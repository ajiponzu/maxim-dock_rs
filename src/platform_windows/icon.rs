use super::{PlatformError, error};
use crate::core::{DockItem, IconSource, TargetKind};
use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::*,
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        UI::{
            Shell::{
                ASSOCF_IS_PROTOCOL, ASSOCF_NOTRUNCATE, ASSOCSTR_EXECUTABLE, AssocQueryStringW,
                IShellItemImageFactory, SHCreateItemFromParsingName, SHFILEINFOW, SHGFI_ICON,
                SHGFI_LARGEICON, SHGetFileInfoW, SIIGBF_ICONONLY,
            },
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, PWSTR},
};

pub struct IconPixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Must run on a background worker. Explicit builtin icons use UI fallback.
pub fn load_icon(item: &DockItem) -> Result<Option<IconPixels>, String> {
    load_icon_with_browser(item, browser_icon)
}

fn load_icon_with_browser(
    item: &DockItem,
    browser: impl FnOnce(&str) -> Result<IconPixels, String>,
) -> Result<Option<IconPixels>, String> {
    if let IconSource::File { path } = &item.icon {
        let custom = (|| {
            let mut reader = image::ImageReader::open(path)
                .map_err(|e| e.to_string())?
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(4096);
            limits.max_image_height = Some(4096);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode().map_err(|e| e.to_string())?;
            let image = if image.width() > 256 || image.height() > 256 {
                image.thumbnail(256, 256)
            } else {
                image
            }
            .into_rgba8();
            Ok::<_, String>(IconPixels {
                width: image.width() as usize,
                height: image.height() as usize,
                rgba: image.into_raw(),
            })
        })();
        match custom {
            Ok(image) => return Ok(Some(image)),
            Err(_) => {
                tracing::warn!(item_id = %item.id, "custom icon unavailable; trying Shell/fallback")
            }
        }
    }
    if matches!(item.icon, IconSource::Builtin { .. }) {
        return Ok(None);
    }
    if item.kind == TargetKind::Url {
        return match browser(&item.target) {
            Ok(icon) => Ok(Some(icon)),
            Err(_) => {
                tracing::warn!(item_id = %item.id, "default browser icon unavailable; using globe");
                Ok(None)
            }
        };
    }
    shell_icon(&item.target)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn browser_icon(target: &str) -> Result<IconPixels, String> {
    let url = url::Url::parse(target).map_err(|e| e.to_string())?;
    let executable = default_browser_executable(url.scheme()).map_err(|e| e.to_string())?;
    shell_icon(&executable).map_err(|e| e.to_string())
}

fn default_browser_executable(scheme: &str) -> Result<String, PlatformError> {
    if !matches!(scheme, "http" | "https") {
        return Err(PlatformError::Invalid("Unsupported browser protocol"));
    }
    // SAFETY: called on the icon worker; balance S_OK and S_FALSE on this thread.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|e| error("CoInitializeEx(browser association)", e))?;
    let _com = Com;
    let protocol: Vec<u16> = scheme.encode_utf16().chain(Some(0)).collect();
    let flags = ASSOCF_IS_PROTOCOL | ASSOCF_NOTRUNCATE;
    let mut length = 0u32;
    // SAFETY: terminated protocol, null output requests size, writable count.
    unsafe {
        AssocQueryStringW(
            flags,
            ASSOCSTR_EXECUTABLE,
            PCWSTR(protocol.as_ptr()),
            PCWSTR::null(),
            None,
            &mut length,
        )
    }
    .ok()
    .map_err(|e| error("AssocQueryStringW(browser size)", e))?;
    if !(2..=32768).contains(&length) {
        return Err(PlatformError::Invalid("Invalid browser association length"));
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: sized UTF-16 output with matching count. API retains no pointers.
    unsafe {
        AssocQueryStringW(
            flags,
            ASSOCSTR_EXECUTABLE,
            PCWSTR(protocol.as_ptr()),
            PCWSTR::null(),
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut length,
        )
    }
    .ok()
    .map_err(|e| error("AssocQueryStringW(browser executable)", e))?;
    let end = buffer
        .iter()
        .position(|&c| c == 0)
        .filter(|&end| end > 0)
        .ok_or(PlatformError::Invalid(
            "Empty or unterminated browser association",
        ))?;
    String::from_utf16(&buffer[..end])
        .map_err(|_| PlatformError::Invalid("Invalid browser association UTF-16"))
}

struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: balances successful CoInitializeEx on this same worker thread.
        unsafe {
            CoUninitialize();
        }
    }
}
struct OwnedIcon(HICON);
impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: SHGFI_ICON transfers an owned, non-shared HICON.
        if let Err(e) = unsafe { DestroyIcon(self.0) } {
            tracing::error!(%e, "DestroyIcon failed");
        }
    }
}
struct Bitmap(HBITMAP);
impl Drop for Bitmap {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            // SAFETY: GetIconInfo/GetImage allocate unselected bitmaps owned by this adapter.
            if !unsafe { DeleteObject(self.0.into()) }.as_bool() {
                tracing::error!("DeleteObject(icon bitmap) failed");
            }
        }
    }
}
struct Dc(HDC);
impl Drop for Dc {
    fn drop(&mut self) {
        // SAFETY: DC was created by CreateCompatibleDC, no selected owned objects.
        if !unsafe { DeleteDC(self.0) }.as_bool() {
            tracing::error!("DeleteDC(icon) failed");
        }
    }
}

fn shell_icon(target: &str) -> Result<IconPixels, PlatformError> {
    if target.contains('\0') || target.encode_utf16().count() >= 260 {
        return Err(PlatformError::Invalid(
            "Shell icon path is invalid or exceeds MAX_PATH",
        ));
    }
    // SAFETY: fresh worker thread apartment; HRESULT S_FALSE also requires balance.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|e| error("CoInitializeEx(icon)", e))?;
    let _com = Com;
    let target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    prefer_high_resolution(image_factory_icon(&target), || large_shell_icon(&target))
}

fn prefer_high_resolution(
    image: Result<IconPixels, PlatformError>,
    fallback: impl FnOnce() -> Result<IconPixels, PlatformError>,
) -> Result<IconPixels, PlatformError> {
    match image {
        Ok(image) => Ok(image),
        Err(e) => {
            tracing::debug!(error = %e, "high-resolution Shell icon unavailable; trying legacy icon");
            fallback()
        }
    }
}

/// Caller owns the initialized COM apartment; this function owns only its bitmap/DC.
fn image_factory_icon(target: &[u16]) -> Result<IconPixels, PlatformError> {
    // SAFETY: terminated path, no bind context; returned COM interface is thread-local RAII.
    let factory: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(PCWSTR(target.as_ptr()), None) }
            .map_err(|e| error("SHCreateItemFromParsingName(icon)", e))?;
    // SAFETY: live interface, bounded request; owned HBITMAP must be DeleteObject'd.
    // ICONONLY avoids document thumbnails; no SCALEUP inventing missing detail.
    let bitmap = Bitmap(
        unsafe { factory.GetImage(SIZE { cx: 256, cy: 256 }, SIIGBF_ICONONLY) }
            .map_err(|e| error("IShellItemImageFactory::GetImage", e))?,
    );
    if bitmap.0.0.is_null() {
        return Err(PlatformError::Invalid(
            "Shell image factory returned no bitmap",
        ));
    }
    let mut info = BITMAP::default();
    // SAFETY: live owned bitmap and correctly sized writable BITMAP.
    if unsafe {
        GetObjectW(
            bitmap.0.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&mut info as *mut BITMAP).cast()),
        )
    } == 0
    {
        return Err(PlatformError::Invalid(
            "GetObjectW(high-resolution icon) failed",
        ));
    }
    let (width, height) = (info.bmWidth, info.bmHeight);
    if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
        return Err(PlatformError::Invalid(
            "Unsupported high-resolution icon dimensions",
        ));
    }
    // SAFETY: memory DC with no selected owned objects.
    let dc = Dc(unsafe { CreateCompatibleDC(None) });
    if dc.0.0.is_null() {
        return Err(PlatformError::Invalid(
            "CreateCompatibleDC(high-resolution icon) failed",
        ));
    }
    let mut pixels = bitmap_pixels(&dc, bitmap.0, width, height)?;
    if pixels.chunks_exact(4).all(|p| p[3] == 0) {
        return Err(PlatformError::Invalid(
            "Shell bitmap has no alpha; trying legacy icon mask",
        ));
    }
    unpremultiply_bgra(&mut pixels);
    tracing::debug!(width, height, "high-resolution Shell icon loaded");
    Ok(IconPixels {
        width: width as usize,
        height: height as usize,
        rgba: pixels,
    })
}

fn large_shell_icon(target: &[u16]) -> Result<IconPixels, PlatformError> {
    let mut info = SHFILEINFOW::default();
    // SAFETY: terminated UTF-16 and correctly sized writable structure. No retained pointer.
    let status = unsafe {
        SHGetFileInfoW(
            PCWSTR(target.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if status == 0 || info.hIcon.0.is_null() {
        return Err(PlatformError::Invalid("Shell could not extract an icon"));
    }
    let icon = OwnedIcon(info.hIcon);
    let mut bits = ICONINFO::default();
    // SAFETY: live icon, writable ICONINFO; its newly allocated bitmaps become RAII-owned below.
    unsafe { GetIconInfo(icon.0, &mut bits) }.map_err(|e| error("GetIconInfo", e))?;
    let color = Bitmap(bits.hbmColor);
    let mask = Bitmap(bits.hbmMask);
    if color.0.0.is_null() {
        return Err(PlatformError::Invalid("Monochrome icon; using fallback"));
    }
    let mut bitmap = BITMAP::default();
    // SAFETY: live bitmap and correctly sized output buffer.
    if unsafe {
        GetObjectW(
            color.0.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&mut bitmap as *mut BITMAP).cast()),
        )
    } == 0
    {
        return Err(PlatformError::Invalid("GetObjectW(icon) failed"));
    }
    let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
    if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
        return Err(PlatformError::Invalid("Unsupported Shell icon dimensions"));
    }
    // SAFETY: memory DC, not tied to any window or borrowed UI resource.
    let dc = Dc(unsafe { CreateCompatibleDC(None) });
    if dc.0.0.is_null() {
        return Err(PlatformError::Invalid("CreateCompatibleDC(icon) failed"));
    }
    let mut pixels = bitmap_pixels(&dc, color.0, width, height)?;
    if pixels.chunks_exact(4).all(|p| p[3] == 0) {
        let mask_pixels = bitmap_pixels(&dc, mask.0, width, height)?;
        for (pixel, mask) in pixels.chunks_exact_mut(4).zip(mask_pixels.chunks_exact(4)) {
            pixel[3] = if mask[0] == 0 { 255 } else { 0 };
        }
    }
    unpremultiply_bgra(&mut pixels);
    Ok(IconPixels {
        width: width as usize,
        height: height as usize,
        rgba: pixels,
    })
}

fn unpremultiply_bgra(pixels: &mut [u8]) {
    // Win32 color channels are BGRA and alpha icons are premultiplied.
    for p in pixels.chunks_exact_mut(4) {
        p.swap(0, 2);
        if p[3] > 0 && p[3] < 255 {
            let alpha = p[3] as u32;
            for channel in &mut p[..3] {
                *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
}

fn bitmap_pixels(
    dc: &Dc,
    bitmap: HBITMAP,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, PlatformError> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    // SAFETY: unselected live bitmap, bounded buffer of width*height*4, top-down 32bit DIB.
    if unsafe {
        GetDIBits(
            dc.0,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        )
    } != height
    {
        return Err(PlatformError::Invalid("GetDIBits(icon) failed"));
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    static NATIVE_ICON_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn high_resolution_failure_uses_legacy_but_success_skips_it() {
        let icon = || IconPixels {
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        };
        assert!(prefer_high_resolution(Ok(icon()), || panic!("must not use fallback")).is_ok());
        let fallback =
            prefer_high_resolution(Err(PlatformError::Invalid("test failure")), || Ok(icon()))
                .unwrap();
        assert_eq!(fallback.rgba, [1, 2, 3, 255]);
        assert!(
            prefer_high_resolution(Err(PlatformError::Invalid("test")), || Err(
                PlatformError::Invalid("legacy failure")
            ))
            .is_err()
        );
    }
    #[test]
    fn bgra_conversion_preserves_alpha_and_unpremultiplies_edges() {
        let mut bytes = [10, 20, 30, 255, 16, 32, 64, 128, 0, 0, 0, 0];
        unpremultiply_bgra(&mut bytes);
        assert_eq!(bytes, [30, 20, 10, 255, 128, 64, 32, 128, 0, 0, 0, 0]);
    }
    #[test]
    fn native_factory_returns_large_icons_for_executable_folder_and_document() {
        std::thread::spawn(|| {
            let _lock = NATIVE_ICON_TEST_LOCK.lock().unwrap();
            let directory = tempfile::tempdir().unwrap();
            let file = directory.path().join("日本語.txt");
            std::fs::write(&file, b"icon test").unwrap();
            let explorer =
                std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("explorer.exe");
            // SAFETY: test worker owns and balances its COM apartment.
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                .ok()
                .unwrap();
            let _com = Com;
            for (name, path) in [
                ("exe", explorer.as_path()),
                ("folder", directory.path()),
                ("document", file.as_path()),
            ] {
                let target: Vec<u16> = path
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                let large = image_factory_icon(&target).unwrap();
                let legacy = large_shell_icon(&target).unwrap();
                assert_eq!((large.width, large.height), (256, 256));
                assert!(large.width > legacy.width);
                assert!(large.rgba.chunks_exact(4).any(|p| p[3] != 0));
                println!(
                    "HIGH_RES_ICON_PASS: {name} {}x{} (legacy {}x{})",
                    large.width, large.height, legacy.width, legacy.height
                );
            }
        })
        .join()
        .unwrap();
    }
    #[test]
    fn custom_png_has_priority_and_missing_custom_url_uses_browser() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("icon.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let mut item = DockItem::new("URL", "https://example.com", TargetKind::Url);
        item.icon = IconSource::File { path: path.clone() };
        let image = load_icon_with_browser(&item, |_| panic!("custom image must win"))
            .unwrap()
            .unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(&image.rgba[..4], &[1, 2, 3, 255]);
        item.icon = IconSource::File {
            path: directory.path().join("missing.png"),
        };
        let image = load_icon_with_browser(&item, |_| {
            Ok(IconPixels {
                width: 1,
                height: 1,
                rgba: vec![4, 5, 6, 255],
            })
        })
        .unwrap()
        .unwrap();
        assert_eq!(image.rgba, vec![4, 5, 6, 255]);
    }
    #[test]
    fn url_browser_failure_and_explicit_builtin_preserve_globe_fallback() {
        for target in ["http://example.com/", "https://example.com/"] {
            let mut item = DockItem::new("URL", target, TargetKind::Url);
            assert!(
                load_icon_with_browser(&item, |received| {
                    assert_eq!(received, target);
                    Err("no association".into())
                })
                .unwrap()
                .is_none()
            );
            item.icon = IconSource::Builtin {
                name: "globe".into(),
            };
            assert!(
                load_icon_with_browser(&item, |_| panic!("explicit builtin must win"))
                    .unwrap()
                    .is_none()
            );
        }
        assert!(default_browser_executable("file").is_err());
    }
    #[test]
    fn native_browser_icons_match_associated_executable() {
        std::thread::spawn(|| {
            let _lock = NATIVE_ICON_TEST_LOCK.lock().unwrap();
            for scheme in ["http", "https"] {
                let executable = default_browser_executable(scheme)
                    .expect("test host needs a registered browser");
                let expected = shell_icon(&executable).unwrap();
                let actual = browser_icon(&format!("{scheme}://example.com/")).unwrap();
                assert_eq!(
                    (actual.width, actual.height),
                    (expected.width, expected.height)
                );
                assert_eq!(actual.rgba, expected.rgba);
                println!(
                    "BROWSER_ICON_PASS: {scheme} {}x{}; association and Shell pixels match",
                    actual.width, actual.height
                );
            }
        })
        .join()
        .unwrap();
    }
    #[test]
    fn repeated_shell_icons_release_gdi_and_user_handles() {
        std::thread::spawn(|| {
            let _lock = NATIVE_ICON_TEST_LOCK.lock().unwrap();
            use windows::Win32::System::Threading::{
                GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetGuiResources,
            };
            let target = std::env::var_os("WINDIR").unwrap();
            let path = std::path::PathBuf::from(target).join("explorer.exe");
            let path = path.to_str().unwrap();
            drop(shell_icon(path).unwrap()); // warm up Shell caches
            // SAFETY: borrowed current-process pseudo handle, count-only diagnostics.
            let counts = || unsafe {
                let process = GetCurrentProcess();
                (
                    GetGuiResources(process, GR_GDIOBJECTS),
                    GetGuiResources(process, GR_USEROBJECTS),
                )
            };
            let before = counts();
            for _ in 0..100 {
                let icon = shell_icon(path).unwrap();
                assert!(icon.rgba.chunks_exact(4).any(|p| p[3] != 0));
            }
            let after = counts();
            assert!(
                after.0 <= before.0 + 2,
                "GDI handles grew: {before:?} -> {after:?}"
            );
            assert!(
                after.1 <= before.1 + 2,
                "USER handles grew: {before:?} -> {after:?}"
            );
        })
        .join()
        .unwrap();
    }
}
