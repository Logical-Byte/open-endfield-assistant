use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "settings/")]
pub enum SettingsErrorKind {
    NewerVersion,
    UnsupportedVersion,
    InvalidFile,
    Validation,
    Read,
    Write,
    Unavailable,
}

/// 面向应用的错误分类与诊断，不携带设置内容或凭据。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "settings/")]
pub struct SettingsError {
    pub kind: SettingsErrorKind,
    pub message: String,
}

impl SettingsError {
    pub fn new(kind: SettingsErrorKind, message: impl std::fmt::Display) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for SettingsError {}
