use anyhow::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetForegroundWindow};
use windows::core::PCWSTR;

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn get_window_by_title(class_name: Option<&str>, title: Option<&str>) -> Result<HWND> {
    let to_pcwstr = |s: &str| -> PCWSTR {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        PCWSTR::from_raw(wide.as_ptr())
    };

    let lpclassname = class_name.map(to_pcwstr);
    let lpwindowname = title.map(to_pcwstr);

    let hwnd = unsafe { FindWindowW(lpclassname.as_ref(), lpwindowname.as_ref()) }?;
    Ok(hwnd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_window() {
        let hwnd = get_foreground_window();
        assert!(!hwnd.is_invalid(), "Failed to get active window");
    }

    #[test]
    fn test_get_window_by_title() {
        let hwnd = get_window_by_title(None, Some("明日方舟"));
        match hwnd {
            Ok(hwnd) => assert!(!hwnd.is_invalid(), "Failed to find window by title"),
            Err(e) => panic!("Error finding window: {:?}", e),
        }
    }
}
