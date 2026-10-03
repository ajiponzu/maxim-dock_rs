//! Bounded, nonblocking diagnostic capture. Never wait for pipe EOF (GUI children may inherit it).
use std::{io::Read, os::windows::io::AsRawHandle, process::Child};
use windows::Win32::{
    Foundation::{ERROR_BROKEN_PIPE, HANDLE},
    System::Pipes::PeekNamedPipe,
};
const LIMIT: usize = 8192;
#[derive(Default)]
pub(super) struct Diagnostics {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
impl Diagnostics {
    pub(super) fn poll(&mut self, child: &mut Child) -> Result<(), String> {
        if let Some(pipe) = &mut child.stdout {
            drain(pipe, &mut self.stdout)?;
        }
        if let Some(pipe) = &mut child.stderr {
            drain(pipe, &mut self.stderr)?;
        }
        Ok(())
    }
    pub(super) fn message(&self) -> String {
        let stderr = decode(&self.stderr);
        let stdout = decode(&self.stdout);
        let text = [stderr, stdout]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            String::new()
        } else {
            format!("\n診断出力（末尾・最大各8KiB）:\n{text}")
        }
    }
}
fn drain(pipe: &mut (impl Read + AsRawHandle), tail: &mut Vec<u8>) -> Result<(), String> {
    // Bound each polling pass so infinite output cannot starve cancellation/timeout checks.
    for _ in 0..16 {
        let mut available = 0;
        // SAFETY: std owns a live anonymous-pipe handle; only query availability, no raw reads.
        let result = unsafe {
            PeekNamedPipe(
                HANDLE(pipe.as_raw_handle()),
                None,
                0,
                None,
                Some(&mut available),
                None,
            )
        };
        if let Err(error) = result {
            return if error.code() == windows::core::HRESULT::from_win32(ERROR_BROKEN_PIPE.0) {
                Ok(())
            } else {
                Err(format!("WSL 診断出力の取得失敗: {error}"))
            };
        }
        if available == 0 {
            break;
        }
        let mut buffer = [0_u8; 4096];
        let size = (available as usize).min(buffer.len());
        let size = pipe
            .read(&mut buffer[..size])
            .map_err(|e| format!("WSL 診断出力の読込失敗: {e}"))?;
        if size == 0 {
            break;
        }
        append(tail, &buffer[..size]);
    }
    Ok(())
}
fn append(tail: &mut Vec<u8>, bytes: &[u8]) {
    tail.extend_from_slice(bytes);
    if tail.len() > LIMIT {
        tail.drain(..tail.len() - LIMIT);
    }
}
fn decode(bytes: &[u8]) -> String {
    // wsl.exe's own errors can be UTF-16LE; Linux stderr is normally UTF-8.
    let utf16 = bytes.starts_with(&[0xff, 0xfe])
        || bytes.chunks_exact(2).take(64).filter(|p| p[1] == 0).count() > 4;
    let text = if utf16 {
        String::from_utf16_lossy(
            &bytes
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>(),
        )
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect::<String>()
        .trim_matches(['\u{feff}', ' ', '\n', '\t'])
        .to_owned()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_tail_is_bounded_and_decodes_linux_and_windows_errors() {
        let mut tail = Vec::new();
        append(&mut tail, &vec![b'x'; LIMIT * 3]);
        append(&mut tail, b" missing code");
        assert_eq!(tail.len(), LIMIT);
        assert!(decode(&tail).ends_with("missing code"));
        assert_eq!(decode("日本語\r\nfailed\x1b".as_bytes()), "日本語\nfailed");
        let utf16: Vec<_> = "WSL: user not found\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(decode(&utf16), "WSL: user not found");
    }
}
