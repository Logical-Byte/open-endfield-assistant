//! 游戏自动化接口。
//!
//! 主要外部接口：
//!
//! 提供 [`TaskKind`]、[`Status`] 和 [`Runtime`] 作为自动化任务的运行时，支持选择任务种类、查询任务的生命状态，并保持最多一个任务同时运行。这些自动化任务以后台线程的形式异步运行。
//!
//! 对于具体的自动化任务工作流，[`capabilities`] 模块提供了输入、截图、OCR、模板匹配和时钟等游戏操作能力作为 trait。
//!
//! 子模块还封装了这些内容：
//! - 自动化任务生命周期管理（打断、观测、查询）。
//! - [`capabilities`] 的 adapter。
//! - 具体的自动化任务工作流实现。

pub(crate) mod archive_scan;
pub(crate) mod cancellation;
mod capabilities;
pub(crate) mod runtime;
pub(crate) mod session;
pub mod stats;

pub(crate) use cancellation::{
    AutomationStopped, StopToken, is_stop_requested, new_stop_token, request_stop,
};
pub use capabilities::{
    Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
    TemplateTarget,
};
pub(crate) use runtime::Runtime;
pub use runtime::Status;

/// 用户可以启动的自动化任务种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    ArchiveScan,
}
