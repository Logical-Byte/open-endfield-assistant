//! 更新下载相关的 Tauri 命令及其序列化接口。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::controller::Controller;

use super::{UpdateManager, check, download, http, source};

/// 前端可见的可用更新信息。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    version_name: String,
    release_note: String,
}

/// 更新可用性。第三方下载元数据只保留在后端缓存中。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UpdateAvailability {
    UpToDate,
    Available { update: AvailableUpdate },
}

/// 一次高层更新下载的进度。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub(super) downloaded_size: u64,
    pub(super) total_size: u64,
    pub(super) speed: u64,
    pub(super) progress: f64,
}

/// 后端完成原子发布后的可安装更新。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedUpdate {
    downloaded_package_path: String,
    version_name: String,
    release_note: String,
}

/// 检查更新并缓存规范化后的可用更新。
#[tauri::command]
pub async fn check_update(
    manager: tauri::State<'_, UpdateManager>,
    controller: tauri::State<'_, Arc<Controller>>,
    app: tauri::AppHandle,
) -> Result<UpdateAvailability, String> {
    let check_lease = manager.start_check().map_err(|error| error.to_string())?;
    let config = controller
        .oea_config()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let current_version = app.package_info().version.to_string();
    let user_agent = http::update_user_agent(&current_version);
    let available = check::check_for_update(&config, &current_version, &user_agent).await?;

    match available {
        Some(metadata) => {
            let result = UpdateAvailability::Available {
                update: AvailableUpdate {
                    version_name: metadata.version_name.clone(),
                    release_note: metadata.release_note.clone(),
                },
            };
            check_lease.complete(Some(metadata));
            Ok(result)
        }
        None => {
            check_lease.complete(None);
            Ok(UpdateAvailability::UpToDate)
        }
    }
}

/// 使用缓存的可用更新和当前配置完成更新包下载。
#[tauri::command]
pub async fn download_update(
    manager: tauri::State<'_, UpdateManager>,
    controller: tauri::State<'_, Arc<Controller>>,
    app: tauri::AppHandle,
    on_progress: tauri::ipc::Channel<DownloadProgress>,
) -> Result<DownloadedUpdate, String> {
    let download_lease = manager
        .start_update_download()
        .map_err(|error| error.to_string())?;
    let config = controller
        .oea_config()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let metadata = download_lease
        .available_update()
        .expect("高层下载 lease 必须包含可用更新")
        .clone();
    let session = download_lease.session();
    let cancellation = session.cancellation();
    let user_agent = http::update_user_agent(&app.package_info().version.to_string());
    let plan =
        source::resolve_download_plan(&metadata, &config, &user_agent, &cancellation).await?;
    let path = download::download_update_plan(
        plan,
        download_lease.id(),
        session,
        &config,
        &user_agent,
        on_progress,
    )
    .await?;

    Ok(DownloadedUpdate {
        downloaded_package_path: path,
        version_name: metadata.version_name,
        release_note: metadata.release_note,
    })
}

/// 下载进度事件（前端按 `session_id` 过滤旧任务的迟到事件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    /// 下载会话编号（自增，用于区分新旧任务）。
    pub(super) session_id: u64,
    /// 已下载字节数。
    pub(super) downloaded_size: u64,
    /// 总字节数（未知时为 `0`）。
    pub(super) total_size: u64,
    /// EMA 平滑后的瞬时速度（字节/秒）。
    pub(super) speed: u64,
    /// 进度百分比（`0.0` ~ `100.0`，总大小未知时为 `0.0`）。
    pub(super) progress: f64,
}

/// 下载结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    /// 本次下载的会话编号。
    pub(super) session_id: u64,
    /// 实际保存路径（可能因响应中的文件名而不同于请求值）。
    pub(super) actual_save_path: String,
    /// 从响应中检测到的文件名。
    pub(super) detected_filename: Option<String>,
}

/// 文件下载命令的请求参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub(super) url: String,
    pub(super) save_path: String,
    pub(super) total_size: Option<u64>,
    pub(super) expected_sha256: Option<String>,
    pub(super) proxy_mode: Option<ProxyMode>,
    pub(super) proxy_url: Option<String>,
    pub(super) accept: Option<String>,
    pub(super) user_agent: Option<String>,
}

/// 文件下载使用的代理模式。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// 不使用代理。
    None,
    /// 使用操作系统代理。
    System,
    /// 使用请求中提供的代理 URL。
    Custom,
}

/// 下载文件。
#[tauri::command]
pub async fn download_file(
    manager: tauri::State<'_, UpdateManager>,
    app: tauri::AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    download::download(&manager, app, request).await
}

/// 取消当前文件下载。
#[tauri::command]
pub fn cancel_download(manager: tauri::State<'_, UpdateManager>) -> Result<(), String> {
    manager.cancel_download().map_err(|error| error.to_string())
}

/// 返回文件下载目录。
#[tauri::command]
pub fn get_download_dir() -> Result<String, String> {
    download::get_download_dir()
}

/// 返回更新检查和下载共用的 Windows 系统代理。
#[tauri::command]
pub fn resolve_system_proxy() -> Result<Option<String>, String> {
    download::resolve_system_proxy()
}

#[cfg(test)]
mod tests {
    use super::{AvailableUpdate, DownloadProgress, DownloadedUpdate, UpdateAvailability};

    #[test]
    fn update_availability_has_a_tagged_camel_case_contract() {
        assert_eq!(
            serde_json::to_value(UpdateAvailability::UpToDate).unwrap(),
            serde_json::json!({ "status": "upToDate" })
        );
        assert_eq!(
            serde_json::to_value(UpdateAvailability::Available {
                update: AvailableUpdate {
                    version_name: "v1.3.0".to_string(),
                    release_note: "notes".to_string(),
                },
            })
            .unwrap(),
            serde_json::json!({
                "status": "available",
                "update": { "versionName": "v1.3.0", "releaseNote": "notes" }
            })
        );
    }

    #[test]
    fn download_progress_has_only_the_four_camel_case_ui_fields() {
        assert_eq!(
            serde_json::to_value(DownloadProgress {
                downloaded_size: 10,
                total_size: 20,
                speed: 30,
                progress: 50.0,
            })
            .unwrap(),
            serde_json::json!({
                "downloadedSize": 10,
                "totalSize": 20,
                "speed": 30,
                "progress": 50.0,
            })
        );
    }

    #[test]
    fn downloaded_update_has_the_public_result_fields() {
        assert_eq!(
            serde_json::to_value(DownloadedUpdate {
                downloaded_package_path: "C:\\OEA\\cache\\downloads\\update.zip".to_string(),
                version_name: "v1.3.0".to_string(),
                release_note: "notes".to_string(),
            })
            .unwrap(),
            serde_json::json!({
                "downloadedPackagePath": "C:\\OEA\\cache\\downloads\\update.zip",
                "versionName": "v1.3.0",
                "releaseNote": "notes",
            })
        );
    }
}
