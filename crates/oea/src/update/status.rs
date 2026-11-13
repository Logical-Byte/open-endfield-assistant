//! 更新状态的前端传输类型。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 前端可见的更新展示信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub struct UpdateInfo {
    pub(super) version_name: String,
    pub(super) release_note: String,
}

/// 前端可见的更新操作状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub enum UpdateOperationStatus {
    Idle,
    Checking,
    Downloading,
    Installing,
}

/// 下载来源跟随后端会话保存，WebView 重载不能改变手动安装的确认边界。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub enum DownloadTrigger {
    #[default]
    Manual,
    Automatic,
}

/// 更新器状态与后端下载来源的前端快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub struct UpdateStatus {
    pub(super) operation: UpdateOperationStatus,
    pub(super) available_update: Option<UpdateInfo>,
    pub(super) pending_update: Option<UpdateInfo>,
    pub(super) download_trigger: DownloadTrigger,
}

impl From<oea_update::UpdateInfo> for UpdateInfo {
    fn from(info: oea_update::UpdateInfo) -> Self {
        Self {
            version_name: info.version_name,
            release_note: info.release_note,
        }
    }
}
impl From<oea_update::UpdateOperationStatus> for UpdateOperationStatus {
    fn from(status: oea_update::UpdateOperationStatus) -> Self {
        match status {
            oea_update::UpdateOperationStatus::Idle => Self::Idle,
            oea_update::UpdateOperationStatus::Checking => Self::Checking,
            oea_update::UpdateOperationStatus::Downloading => Self::Downloading,
            oea_update::UpdateOperationStatus::Installing => Self::Installing,
        }
    }
}
impl From<oea_update::UpdateStatus> for UpdateStatus {
    fn from(status: oea_update::UpdateStatus) -> Self {
        Self {
            operation: status.operation.into(),
            available_update: status.available_update.map(Into::into),
            pending_update: status.pending_update.map(Into::into),
            download_trigger: DownloadTrigger::Manual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateStatus;

    #[test]
    fn status_conversion_preserves_the_frontend_snapshot() {
        let info = oea_update::UpdateInfo {
            version_name: "1.3.0".to_string(),
            release_note: "notes".to_string(),
        };
        let status = UpdateStatus::from(oea_update::UpdateStatus {
            operation: oea_update::UpdateOperationStatus::Idle,
            available_update: Some(info.clone()),
            pending_update: Some(info),
        });
        let json = serde_json::to_value(status).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "operation": "idle",
                "availableUpdate": { "versionName": "1.3.0", "releaseNote": "notes" },
                "pendingUpdate": { "versionName": "1.3.0", "releaseNote": "notes" },
                "downloadTrigger": "manual"
            })
        );
    }
}
