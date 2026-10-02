//! Windows system integrations. Scripts are fixed; user data travels over stdin.
use std::{io::Write, process::Stdio};
pub fn powershell(script: &str, input: &str) -> Result<String, String> {
    let root = std::env::var_os("SystemRoot").ok_or("SystemRoot is unavailable")?;
    let executable =
        std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = std::process::Command::new(executable);
    mstudio::media::quiet(&mut command);
    let mut child = command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-Command",
            script,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("System command stdin unavailable")?
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Windows system command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .into())
}

pub fn clipboard_paths() -> Vec<std::path::PathBuf> {
    use std::{ffi::c_void, os::windows::ffi::OsStringExt};
    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(window: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn GetClipboardData(format: u32) -> *mut c_void;
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn DragQueryFileW(drop: *mut c_void, index: u32, file: *mut u16, size: u32) -> u32;
    }
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    // CF_HDROP supplies native file references, never copied plain text paths.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return vec![];
        }
        let _clipboard = Clipboard;
        let drop = GetClipboardData(15);
        if drop.is_null() {
            return vec![];
        }
        let count = DragQueryFileW(drop, u32::MAX, std::ptr::null_mut(), 0).min(1000);
        (0..count)
            .filter_map(|index| {
                let length = DragQueryFileW(drop, index, std::ptr::null_mut(), 0);
                if length == 0 || length > 32767 {
                    return None;
                }
                let mut text = vec![0u16; length as usize + 1];
                let read = DragQueryFileW(drop, index, text.as_mut_ptr(), text.len() as u32);
                (read == length)
                    .then(|| std::ffi::OsString::from_wide(&text[..read as usize]).into())
            })
            .collect()
    }
}
