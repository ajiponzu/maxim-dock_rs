use super::{PlatformError, error};
use crate::core::{DockItem, IconSource, TargetKind};
use windows::{
    Win32::{
        Graphics::Gdi::*,
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        UI::{
            Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW},
            WindowsAndMessaging::*,
        },
    },
    core::PCWSTR,
};

pub struct IconPixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Must run on a background worker. URL and explicit builtin icons use UI fallback.
pub fn load_icon(item: &DockItem) -> Result<Option<IconPixels>, String> {
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
    if matches!(item.icon, IconSource::Builtin { .. }) || item.kind == TargetKind::Url {
        return Ok(None);
    }
    shell_icon(&item.target)
        .map(Some)
        .map_err(|e| e.to_string())
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
            // SAFETY: GetIconInfo allocates unselected bitmaps owned by this adapter.
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
    Ok(IconPixels {
        width: width as usize,
        height: height as usize,
        rgba: pixels,
    })
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
    #[test]
    fn custom_png_has_priority_and_missing_custom_url_uses_builtin() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("icon.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let mut item = DockItem::new("URL", "https://example.com", TargetKind::Url);
        item.icon = IconSource::File { path: path.clone() };
        let image = load_icon(&item).unwrap().unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(&image.rgba[..4], &[1, 2, 3, 255]);
        item.icon = IconSource::File {
            path: directory.path().join("missing.png"),
        };
        assert!(load_icon(&item).unwrap().is_none());
    }
    #[test]
    fn repeated_shell_icons_release_gdi_and_user_handles() {
        std::thread::spawn(|| {
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
