//! 游戏自动化子系统。
//!
//! 档案扫描、场景导航和路由执行只依赖本模块中的细粒度 capability trait，避免了解
//! [`Session`](session::Session) 如何持有窗口、截图器、输入器和识别资源。
//! 生产环境由 [`Session`](session::Session) 实现这些能力；测试可以直接实现
//! 相同的能力接口。

pub(crate) mod cancellation;
mod capabilities;
pub(crate) mod scan_runtime;
pub(crate) mod session;

pub(crate) use cancellation::{AutomationStopped, StopToken};
pub use capabilities::{
    Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
    TemplateTarget,
};
