//! 自动化失败事实：生命周期失败与工作流执行失败分别建模。
use serde::Serialize;
use ts_rs::TS;

use crate::{automation::capabilities, navigation};

/// 自动化运行失败所属的职责，由 `scope` 区分：
/// - `runtime`：创建或管理任务运行时失败，其他字段由 `RuntimeError` 定义。
/// - `worker`：工作者执行任务失败，其他字段由 `WorkerError` 定义。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "scope", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "AutomationError")]
pub enum Error {
    Runtime(RuntimeError),
    Worker(WorkerError),
}

impl From<RuntimeError> for Error {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<WorkerError> for Error {
    fn from(error: WorkerError) -> Self {
        Self::Worker(error)
    }
}

/// 自动化生命周期的失败原因，由 `kind` 区分：
/// - `threadStartFailed`：无法创建执行任务的后台线程。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "AutomationRuntimeError")]
pub enum RuntimeError {
    ThreadStartFailed,
}

/// 各自动化工作流可以复用的失败原因，由 `kind` 区分：
/// - `capability`：`reason` 携带能力边界的失败原因，包括用户停止。
/// - `navigationFailed`：无法导航到任务需要的游戏界面。
/// - `custom`：`message` 为工作流自行提供的可读文本，显示时保留原文，不作为翻译 key。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "AutomationWorkerError")]
pub enum WorkerError {
    Capability {
        reason: capabilities::Error,
    },
    NavigationFailed,
    Custom {
        /// 自定义可读原因，显示原文，不作为翻译 key。
        message: String,
    },
}
impl WorkerError {
    pub fn is_stopped_by_user(&self) -> bool {
        matches!(self, Self::Capability { reason } if reason.is_stopped_by_user())
    }
}
impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capability { reason } => write!(f, "{reason}"),
            Self::NavigationFailed => f.write_str("游戏界面导航失败"),
            Self::Custom { message } => f.write_str(message),
        }
    }
}
impl std::error::Error for WorkerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Capability { reason } => Some(reason),
            _ => None,
        }
    }
}

impl From<capabilities::Error> for WorkerError {
    fn from(error: capabilities::Error) -> Self {
        Self::Capability { reason: error }
    }
}

impl From<navigation::Error> for WorkerError {
    fn from(error: navigation::Error) -> Self {
        match error {
            navigation::Error::Capability(error) => Self::from(error),
            navigation::Error::Failed { .. } => Self::NavigationFailed,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Worker(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Worker(error) => Some(error),
        }
    }
}
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("无法创建自动化任务线程")
    }
}
impl std::error::Error for RuntimeError {}
