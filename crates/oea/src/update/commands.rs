//! 更新下载相关的 Tauri 命令及其序列化接口。
use super::{
    Updater,
    error::Reason,
    options,
    status::{UpdateInfo, UpdateStatus},
};
use crate::{app_paths::AppPaths, controller::Controller, update};
use serde::Serialize;
use std::sync::Arc;
use tracing::warn;
use ts_rs::TS;

/// 更新可用性。第三方下载元数据只保留在后端缓存中。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub enum UpdateAvailability {
    UpToDate,
    Available {
        /// 可用更新的公开版本信息。
        update: UpdateInfo,
    },
}

/// 高层更新下载的结果，由 `status` 区分：
/// - `completed`：下载成功并登记为待安装更新，`update` 携带公开版本信息。
/// - `cancelled`：用户主动取消下载，属于正常结束。
///
/// 租约与临时文件仍由原守卫清理。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub enum DownloadOutcome {
    Completed {
        /// 已下载更新的公开版本信息。
        update: UpdateInfo,
    },
    Cancelled,
}

/// 一次高层更新下载的进度。
///
/// Tauri JSON 以 `number` 传输字节数与速度，采用 JavaScript 安全整数范围
/// （不超过 `2^53 - 1`），因此覆盖 `ts-rs` 对 `u64` 的默认 `bigint` 映射。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "update/")]
pub struct DownloadProgress {
    #[ts(type = "number")]
    pub(super) downloaded_size: u64,
    #[ts(type = "number")]
    pub(super) total_size: u64,
    #[ts(type = "number")]
    pub(super) speed: u64,
    pub(super) progress: f64,
}

impl From<oea_update::UpdateAvailability> for UpdateAvailability {
    fn from(availability: oea_update::UpdateAvailability) -> Self {
        match availability {
            oea_update::UpdateAvailability::UpToDate => Self::UpToDate,
            oea_update::UpdateAvailability::Available { update } => Self::Available {
                update: update.into(),
            },
        }
    }
}
impl From<oea_update::DownloadOutcome> for DownloadOutcome {
    fn from(outcome: oea_update::DownloadOutcome) -> Self {
        match outcome {
            oea_update::DownloadOutcome::Completed { update } => Self::Completed {
                update: update.into(),
            },
            oea_update::DownloadOutcome::Cancelled => Self::Cancelled,
        }
    }
}
impl From<oea_update::DownloadProgress> for DownloadProgress {
    fn from(progress: oea_update::DownloadProgress) -> Self {
        Self {
            downloaded_size: progress.downloaded_size,
            total_size: progress.total_size,
            speed: progress.speed,
            progress: progress.progress,
        }
    }
}

/// 读取可用更新、待安装更新和当前操作的一致快照。
#[tauri::command]
pub fn get_update_status(manager: tauri::State<'_, Updater>) -> UpdateStatus {
    manager.status().into()
}

/// 检查更新并缓存规范化后的可用更新。
#[tauri::command]
pub async fn check_update(
    manager: tauri::State<'_, Updater>,
    controller: tauri::State<'_, Controller>,
    app: tauri::AppHandle,
) -> Result<UpdateAvailability, update::Error> {
    update::ensure_enabled()?;
    let settings = controller
        .settings_snapshot()
        .map_err(|error| update::Error::failed(Reason::SettingsUnavailable, error.into()))?;
    let options = options::check_options(&settings)?;
    manager
        .check(&app.package_info().version.to_string(), || Ok(options))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// 使用缓存的可用更新和当前设置完成更新包下载。
#[tauri::command]
pub async fn download_update(
    manager: tauri::State<'_, Updater>,
    controller: tauri::State<'_, Controller>,
    app: tauri::AppHandle,
    on_progress: tauri::ipc::Channel<DownloadProgress>,
) -> Result<DownloadOutcome, update::Error> {
    update::ensure_enabled()?;
    let downloads = AppPaths::new()?.downloads_dir();
    let settings = controller
        .settings_snapshot()
        .map_err(|error| update::Error::failed(Reason::SettingsUnavailable, error.into()))?;
    let source = options::download_source(&settings)?;
    manager
        .download(
            &app.package_info().version.to_string(),
            &downloads,
            |_| Ok(source),
            Arc::new(move |progress| {
                on_progress
                    .send(DownloadProgress::from(progress))
                    .map_err(|error| error.to_string())
            }),
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

/// 取消当前文件下载。
#[tauri::command]
pub fn cancel_download(manager: tauri::State<'_, Updater>) -> Result<(), update::Error> {
    manager.cancel_download().inspect_err(|cancel_error| {
        warn!(operation = "cancel_download", error = %cancel_error, "取消更新下载请求被状态机拒绝");
    }).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{DownloadOutcome, DownloadProgress, UpdateAvailability};

    #[test]
    fn update_availability_has_a_tagged_camel_case_contract() {
        assert_eq!(
            serde_json::to_value(UpdateAvailability::from(
                oea_update::UpdateAvailability::UpToDate
            ))
            .unwrap(),
            serde_json::json!({ "status": "upToDate" })
        );
        assert_eq!(
            serde_json::to_value(UpdateAvailability::from(
                oea_update::UpdateAvailability::Available {
                    update: oea_update::UpdateInfo {
                        version_name: "v1.3.0".to_string(),
                        release_note: "notes".to_string(),
                    },
                }
            ))
            .unwrap(),
            serde_json::json!({
                "status": "available",
                "update": { "versionName": "v1.3.0", "releaseNote": "notes" }
            })
        );
    }

    #[test]
    fn download_results_and_progress_keep_the_frontend_contract() {
        assert_eq!(
            serde_json::to_value(DownloadOutcome::from(
                oea_update::DownloadOutcome::Completed {
                    update: oea_update::UpdateInfo {
                        version_name: "v1.3.0".into(),
                        release_note: "notes".into()
                    },
                }
            ))
            .unwrap(),
            serde_json::json!({ "status": "completed", "update": { "versionName": "v1.3.0", "releaseNote": "notes" } })
        );
        assert_eq!(
            serde_json::to_value(DownloadOutcome::from(
                oea_update::DownloadOutcome::Cancelled
            ))
            .unwrap(),
            serde_json::json!({ "status": "cancelled" })
        );
        let progress = DownloadProgress::from(oea_update::DownloadProgress {
            downloaded_size: 4_294_967_297,
            total_size: 8_589_934_594,
            speed: 1024,
            progress: 50.0,
        });
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            serde_json::json!({ "downloadedSize": 4_294_967_297_u64, "totalSize": 8_589_934_594_u64, "speed": 1024, "progress": 50.0 })
        );
    }
}
