//! 游戏自动化接口。
//!
//! 主要外部接口：
//!
//! 提供 [`TaskKind`]、[`Status`] 和 [`Runtime`] 作为自动化任务的运行时，支持选择任务种类、查询任务的生命状态，并保持最多一个任务同时运行。这些自动化任务以后台线程的形式异步运行。
//!
//! 对于具体的自动化任务工作流，[`capabilities`] 模块提供了输入、截图、OCR、模板匹配和时钟等游戏操作能力作为 trait。这些调用可以改变游戏状态，或为工作流控制提供输入。
//! [`events`] 模块提供事件观察出口，发布 [`Runtime`] 和自动化任务中产生的领域事实，不参与工作流控制。生产环境通过 Tauri 事件将这些事实传递给前端。
//!
//! 子模块还封装了这些内容：
//! - 自动化任务生命周期管理（打断、观测、查询）。
//! - [`capabilities`] 的 adapter。
//! - 具体的自动化任务工作流实现。

use ts_rs::TS;

pub(crate) mod archive_scan;
mod cancellation;
mod capabilities;
mod events;
mod runtime;
mod session;
#[cfg(feature = "cli")]
pub(crate) use session::Session;
#[cfg(feature = "cli")]
pub(crate) use session::normalize_screenshot;
mod stats;

pub(crate) use cancellation::new_stop_token;
use cancellation::{AutomationStopped, StopToken, is_stop_requested, request_stop};
pub use capabilities::{
    Clock, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
    TemplateTarget,
};
pub(crate) use events::{Event, EventSink};
pub(crate) use runtime::Runtime;
pub use runtime::{LastRun, RunOutcome, Status};

/// 用户可以启动的自动化任务种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "automation/")]
pub enum TaskKind {
    ArchiveScan,
}

#[derive(serde::Deserialize, TS)]
#[serde(tag = "taskKind", rename_all = "camelCase")]
#[ts(export, export_to = "automation/")]
pub(crate) enum StartRequest {
    ArchiveScan {
        #[serde(rename = "workerType")]
        worker_type: archive_scan::WorkerType,
    },
}
