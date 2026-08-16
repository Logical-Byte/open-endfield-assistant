//! 截图器模块（基础设施，通用库）。
//!
//! 通过 [`ScreencapBase`] trait 抽象截图能力，`Session` 持有 `Box<dyn ScreencapBase>`，
//! 运行时可切换（见 `Session::set_screencap`）。当前生产使用 [`PrintWindowScreencap`]，
//! 其余实现保留作为备选 / 实验（未来可切换）。
//!
//! 各实现的适用场景：
//! - [`PrintWindowScreencap`]：`PrintWindow` API 捕获窗口客户区
//!   （`PW_CLIENTONLY | PW_RENDERFULLCONTENT`，可捕获非最小化后台窗口）——**生产使用**；
//! - [`GdiScreencap`]：GDI（`GetDC` + `BitBlt`）捕获窗口客户区；
//! - [`ScreenDCScreencap`]：将窗口客户区坐标换算到屏幕后，从屏幕 DC 捕获；
//! - [`DesktopDupScreencap`]：Desktop Duplication API 捕获整屏
//!   （窗口跨显示器移动时自动重连）；
//! - [`DesktopDupWindowScreencap`]：Desktop Duplication 全屏后按窗口客户区裁剪；
//! - [`FramePoolScreencap`]：DXGI 帧池（`Direct3D11CaptureFramePool`）捕获窗口，
//!   窗口尺寸变化时自动重建。

mod base;
mod desktop_dup;
mod desktop_dup_window;
mod frame_pool;
mod gdi;
mod print_window;
mod screen_dc;

pub use base::*;
pub use desktop_dup::*;
pub use desktop_dup_window::*;
pub use frame_pool::*;
pub use gdi::*;
pub use print_window::*;
pub use screen_dc::*;
