//! 能力边界的失败原因。底层诊断在能力实现中记录。
use crate::automation::GameEnvironmentError;
use serde::Serialize;
use ts_rs::TS;

/// 能力失败由 `kind` 区分。`stoppedByUser` 表示用户停止，`gameEnvironment` 的
/// `reason` 携带游戏环境事实。截图和操作失败的底层诊断保留在日志。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "errors/", rename = "AutomationCapabilityError")]
pub enum Error {
    StoppedByUser,
    GameEnvironment { reason: GameEnvironmentError },
    CaptureFailed,
    ExecutionFailed,
}

impl Error {
    pub fn is_stopped_by_user(&self) -> bool {
        matches!(self, Self::StoppedByUser)
    }
}

impl From<GameEnvironmentError> for Error {
    fn from(error: GameEnvironmentError) -> Self {
        Self::GameEnvironment { reason: error }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoppedByUser => f.write_str("自动化任务已被用户停止"),
            Self::GameEnvironment { reason } => write!(f, "{reason}"),
            Self::CaptureFailed => f.write_str("游戏截图失败"),
            Self::ExecutionFailed => f.write_str("游戏操作失败"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StoppedByUser | Self::CaptureFailed | Self::ExecutionFailed => None,
            Self::GameEnvironment { reason } => Some(reason),
        }
    }
}
