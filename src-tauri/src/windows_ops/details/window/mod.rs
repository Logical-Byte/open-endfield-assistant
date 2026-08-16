//! Win32 窗口工具（基础设施，通用库）。
//!
//! 全部为自由函数，按用途拆分到各文件：
//! - `foreground`：前台窗口判定（OEA / 终末地），供全局热键前台规则使用；
//! - `get_window`：按标题 / 类名查找窗口、获取前台窗口；
//! - `window_info`：窗口信息查询（标题 / 类名 / 客户区矩形 / 坐标转换）；
//! - `window_operation`：窗口操作（前台置顶、确保在屏、DPI 感知等）；
//! - `hdr`：判断窗口所在显示器是否开启了 HDR。

mod get_window;
pub(in crate::windows_ops) mod hdr;
#[allow(dead_code)]
mod window_info;
mod window_operation;

pub(in crate::windows_ops) use get_window::{
    get_app_window, get_foreground_window, get_window_by_title,
};
pub(in crate::windows_ops) use window_info::get_client_rect;
pub(in crate::windows_ops) use window_operation::{
    ensure_foreground_and_topmost, ensure_window_on_screen, restore_window_if_minimized,
    set_thread_dpi_awareness_context,
};
