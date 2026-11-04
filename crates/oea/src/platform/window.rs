//! OEA 主窗口接入与应用热键的前台判定。

#[cfg(windows)]
use super::windows;
#[cfg(windows)]
use crate::automation::game_environment;

/// 全局热键仅在 OEA 主窗口或游戏窗口位于前台时生效。
pub struct ForegroundGuard {
    #[cfg(windows)]
    state: windows::window::AppWindow,
}

impl ForegroundGuard {
    pub fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self {
                state: windows::window::AppWindow::new(app_handle)?,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = app_handle;
            Ok(Self {})
        }
    }

    pub fn is_foreground_eligible(&self) -> bool {
        #[cfg(windows)]
        {
            if self.state.is_foreground() {
                return true;
            }
            oea_window::search(game_environment::window_query())
                .ok()
                .flatten()
                .and_then(|window| window.is_foreground().ok())
                .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

/// 设置 OEA 应用线程的 DPI 上下文。游戏窗口操作由 `oea-window` 自行管理。
pub fn set_thread_dpi_awareness_context() {
    #[cfg(windows)]
    windows::window::set_thread_dpi_awareness_context();
}
