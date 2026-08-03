//! 前台窗口守卫：判断全局热键触发时前台窗口是否为 OEA 自身或终末地游戏窗口。
//!
//! 分号 / 引号热键仅在 OEA 窗口或终末地游戏窗口位于前台时才响应，
//! 避免在其他程序中误触发扫描 / 启动；`Alt+Delete` 退出热键全局生效，
//! 不受本守卫限制。

use windows::Win32::Foundation::HWND;

use crate::window::{get_active_window, get_window_by_title};

/// 终末地游戏窗口标题
pub const ENDFIELD_WINDOW_TITLE: &str = "Endfield";
/// 终末地游戏窗口类名（Unity）
pub const ENDFIELD_WINDOW_CLASS: &str = "UnityWndClass";

/// 前台窗口守卫。
///
/// 仅持有窗口句柄（裸指针封装），访问被串行化到热键监听线程；
/// 与 [`crate::screencap`] 中其它持有 HWND 的类型采用相同的 Send 约定。
pub struct ForegroundGuard {
    /// OEA 自身主窗口句柄
    oea_hwnd: HWND,
}

// HWND 为 `*mut c_void` 封装，本身非 Send，需手动声明（调用方串行化访问）
unsafe impl Send for ForegroundGuard {}

impl ForegroundGuard {
    /// 创建守卫，绑定 OEA 自身主窗口句柄。
    pub fn new(oea_hwnd: HWND) -> Self {
        Self { oea_hwnd }
    }

    /// 判断当前前台窗口是否为 OEA 自身或终末地游戏窗口。
    ///
    /// 终末地窗口每次实时查找（可能尚未打开或被关闭）；
    /// 找不到时视为"不在前台"，仅当确实匹配时返回 `true`。
    pub fn is_foreground_eligible(&self) -> bool {
        let foreground = get_active_window();

        // OEA 自身窗口在前台
        if foreground == self.oea_hwnd {
            return true;
        }

        // 终末地游戏窗口在前台
        match get_window_by_title(ENDFIELD_WINDOW_TITLE, Some(ENDFIELD_WINDOW_CLASS)) {
            Ok(game_hwnd) if !game_hwnd.is_invalid() => foreground == game_hwnd,
            _ => false,
        }
    }
}
