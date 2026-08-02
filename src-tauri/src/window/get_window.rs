use anyhow::{Result, bail};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetForegroundWindow,
};
use windows::core::PCWSTR;

pub fn get_active_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn get_window_by_title(title: &str, class_name: Option<&str>) -> Result<HWND> {
    let to_pcwstr = |s: &str| -> PCWSTR {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        PCWSTR::from_raw(wide.as_ptr())
    };

    let lpclassname = class_name.map(to_pcwstr);
    let lpclassname = lpclassname.as_ref();
    let lpwindowname = to_pcwstr(title);

    let hwnd = unsafe { FindWindowW(lpclassname, lpwindowname) }?;
    if hwnd.is_invalid() {
        bail!("Window with title '{}' not found", title);
    }
    Ok(hwnd)
}
