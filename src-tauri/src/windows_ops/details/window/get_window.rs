use anyhow::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetForegroundWindow};
use windows::core::PCWSTR;

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn get_window_by_title(class_name: Option<&str>, title: Option<&str>) -> Result<HWND> {
    let class_wide = class_name.map(|s| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    });
    let title_wide = title.map(|s| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    });

    let lpclassname = class_wide.map(|v| PCWSTR::from_raw(v.as_ptr()));
    let lpwindowname = title_wide.map(|v| PCWSTR::from_raw(v.as_ptr()));

    let hwnd = unsafe { FindWindowW(lpclassname.as_ref(), lpwindowname.as_ref()) }?;
    Ok(hwnd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a real interactive Windows desktop session"]
    fn test_get_active_window() {
        let hwnd = get_foreground_window();
        assert!(!hwnd.is_invalid(), "Failed to get active window");
    }

    #[test]
    #[ignore = "requires a real interactive Windows desktop session with a window titled 明日方舟"]
    fn test_get_window_by_title() {
        let hwnd = get_window_by_title(None, Some("明日方舟"));
        match hwnd {
            Ok(hwnd) => assert!(!hwnd.is_invalid(), "Failed to find window by title"),
            Err(e) => panic!("Error finding window: {:?}", e),
        }
    }
}
