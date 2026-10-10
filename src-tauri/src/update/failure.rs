//! 内部更新失败保留诊断，取消拥有独立身份，不依赖展示字符串。
use crate::backend_error::UpdateError;

#[derive(Debug)]
pub(super) enum UpdateFailure {
    Cancelled,
    Failed {
        reason: UpdateError,
        diagnostic: String,
    },
}
impl UpdateFailure {
    pub(super) fn failed(reason: UpdateError, diagnostic: impl ToString) -> Self {
        Self::Failed {
            reason,
            diagnostic: diagnostic.to_string(),
        }
    }
    pub(super) fn reason(&self) -> UpdateError {
        match self {
            Self::Failed { reason, .. } => reason.clone(),
            Self::Cancelled => UpdateError::Failed,
        }
    }
}
impl From<String> for UpdateFailure {
    fn from(diagnostic: String) -> Self {
        Self::failed(UpdateError::Failed, diagnostic)
    }
}
impl std::fmt::Display for UpdateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "更新下载已取消"),
            Self::Failed { diagnostic, .. } => write!(f, "{diagnostic}"),
        }
    }
}
