//! 更新下载相关的 Tauri 命令及其序列化接口。

use serde::{Deserialize, Serialize};

use super::{UpdateManager, download};

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
    pub(super) proxy_mode: Option<String>,
    pub(super) proxy_url: Option<String>,
    pub(super) accept: Option<String>,
    pub(super) user_agent: Option<String>,
}

/// 下载文件。
#[tauri::command]
pub async fn download_file(
    manager: tauri::State<'_, UpdateManager>,
    app: tauri::AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    download::run(&manager, app, request).await
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
