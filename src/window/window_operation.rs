use anyhow::{Result, bail};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, HWND_TOP, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow,
    SetWindowPos,
};

// 窗口激活并置顶工具函数（强化版本，用于需要前台的物理输入方式）
// 用于 LegacyEventInput 和 SeizeInput，因为它们使用 SendInput/mouse_event 等物理输入 API
pub fn ensure_foreground_and_topmost(hwnd: HWND) -> Result<()> {
    if hwnd.is_invalid() {
        bail!("hwnd is invalid");
    }

    // 如果窗口不在前台，先将其置顶
    if hwnd != unsafe { GetForegroundWindow() } {
        // 将窗口移到 Z 序顶部
        unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )
        }?;

        // 尝试设置为前台窗口
        unsafe { SetForegroundWindow(hwnd) }.ok()?;
    }

    Ok(())
}
