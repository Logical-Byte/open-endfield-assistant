//! 导航保留原始能力错误，操作上下文由日志记录。
use crate::automation::capabilities;

#[derive(Debug)]
pub enum Error {
    Capability(capabilities::Error),
    Failed { message: String },
}

impl Error {
    pub(super) fn failed(message: impl Into<String>) -> Self {
        Self::Failed {
            message: message.into(),
        }
    }
    pub(crate) fn is_stopped_by_user(&self) -> bool {
        matches!(self, Self::Capability(error) if error.is_stopped_by_user())
    }
}

impl From<capabilities::Error> for Error {
    fn from(error: capabilities::Error) -> Self {
        Self::Capability(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capability(source) => write!(f, "{source}"),
            Self::Failed { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Capability(source) => Some(source),
            Self::Failed { .. } => None,
        }
    }
}
