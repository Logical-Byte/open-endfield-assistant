//! Win32 窗口工具（基础设施，通用库）。
//!
//! 全部为自由函数，按用途拆分到各文件：
//! - `foreground`：前台窗口判定（OEA / 终末地），供全局热键前台规则使用；
//! - `get_window`：按标题 / 类名查找窗口、获取前台窗口；
//! - `window_info`：窗口信息查询（标题 / 类名 / 客户区矩形 / 坐标转换）；
//! - `window_operation`：窗口操作（前台置顶、确保在屏、DPI 感知等）；
//! - `hdr`：判断窗口所在显示器是否开启了 HDR。

pub use foreground::*;
pub use get_window::*;
pub use window_info::*;
pub use window_operation::*;

pub mod dialog;
mod foreground;
mod get_window;
pub mod hdr;
mod window_info;
mod window_operation;
