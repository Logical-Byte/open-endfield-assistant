//! 更新下载相关的 Tauri 命令及其序列化接口。

use std::sync::Arc;

use serde::Serialize;
use tracing::{debug, error, info, warn};

use crate::{config::UpdateProxyMode, controller::Controller};

use super::{
    UpdateManager, check, download, http,
    manager::{UpdateInfo, UpdateStatus},
    source,
};

/// 更新可用性。第三方下载元数据只保留在后端缓存中。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UpdateAvailability {
    UpToDate,
    Available { update: UpdateInfo },
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

/// 读取可用更新、待安装更新和当前操作的一致快照。
#[tauri::command]
pub fn get_update_status(manager: tauri::State<'_, UpdateManager>) -> UpdateStatus {
    manager.status()
}

/// 检查更新并缓存规范化后的可用更新。
#[tauri::command]
pub async fn check_update(
    manager: tauri::State<'_, UpdateManager>,
    controller: tauri::State<'_, Arc<Controller>>,
    app: tauri::AppHandle,
) -> Result<UpdateAvailability, String> {
    let check_lease = manager.start_check().map_err(|check_error| {
        warn!(
            operation = "check",
            error = %check_error,
            "更新检查请求被状态机拒绝"
        );
        check_error.to_string()
    })?;
    let config = controller.oea_config_snapshot();
    let current_version = app.package_info().version.to_string();
    let user_agent = http::update_user_agent(&current_version);
    debug!(
        operation = "check",
        current_version = %current_version,
        configured_source = ?config.update_source,
        proxy_mode = ?config.update_proxy_mode,
        "更新检查开始"
    );
    info!(
        "开始检查更新：当前版本 {}，配置的下载源为 {}，{}",
        current_version,
        source::update_source_label(config.update_source),
        proxy_mode_label(config.update_proxy_mode)
    );
    let available = match check::check_for_update(&config, &current_version, &user_agent).await {
        Ok(available) => available,
        Err(check_error) => {
            error!(
                operation = "check",
                current_version,
                error = %check_error,
                "更新检查失败"
            );
            return Err(check_error);
        }
    };

    match available {
        Some(metadata) => {
            let result = UpdateAvailability::Available {
                update: UpdateInfo {
                    version_name: metadata.version_name.clone(),
                    release_note: metadata.release_note.clone(),
                },
            };
            debug!(
                operation = "check",
                result = "available",
                current_version = %current_version,
                latest_version = %metadata.version_name,
                "更新检查完成"
            );
            info!(
                "发现新版本 {}（当前版本 {}）",
                metadata.version_name, current_version
            );
            check_lease.complete(Some(metadata));
            Ok(result)
        }
        None => {
            debug!(
                operation = "check",
                result = "up_to_date",
                current_version = %current_version,
                "更新检查完成"
            );
            info!("当前已是最新版本 {current_version}");
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
) -> Result<UpdateInfo, String> {
    let download_lease = manager.start_update_download().map_err(|download_error| {
        warn!(
            operation = "download",
            error = %download_error,
            "更新下载请求被状态机拒绝"
        );
        download_error.to_string()
    })?;
    let config = controller.oea_config_snapshot();
    let metadata = download_lease.available_update().clone();
    let session = download_lease.session();
    let session_id = download_lease.id();
    let cancellation = session.cancellation();
    let user_agent = http::update_user_agent(&app.package_info().version.to_string());
    debug!(
        operation = "download",
        session_id,
        version = %metadata.version_name,
        configured_source = ?config.update_source,
        proxy_mode = ?config.update_proxy_mode,
        "更新下载请求已接受"
    );
    info!(
        "开始下载更新 {}：配置的下载源为 {}，{}",
        metadata.version_name,
        source::update_source_label(config.update_source),
        proxy_mode_label(config.update_proxy_mode)
    );
    let plan =
        match source::resolve_download_plan(&metadata, &config, &user_agent, &cancellation).await {
            Ok(plan) => plan,
            Err(download_error) => {
                error!(
                    operation = "download",
                    phase = "resolve_plan",
                    session_id,
                    version = %metadata.version_name,
                    error = %download_error,
                    "更新下载失败"
                );
                return Err(download_error);
            }
        };
    let package_path = match download::download_update_plan(
        plan,
        session_id,
        session,
        &config,
        &user_agent,
        on_progress,
    )
    .await
    {
        Ok(package_path) => package_path,
        Err(download_error) => {
            if download_error == "下载已取消" {
                debug!(
                    operation = "download",
                    result = "cancelled",
                    session_id,
                    version = %metadata.version_name,
                    "更新下载已取消"
                );
                info!("更新 {} 的下载已取消", metadata.version_name);
            } else {
                error!(
                    operation = "download",
                    phase = "transfer",
                    session_id,
                    version = %metadata.version_name,
                    error = %download_error,
                    "更新下载失败"
                );
            }
            return Err(download_error);
        }
    };

    let update = UpdateInfo::from(&metadata);
    debug!(
        operation = "download",
        result = "pending_install",
        session_id,
        version = %metadata.version_name,
        package_path = %package_path.display(),
        "更新包已登记为待安装更新"
    );
    info!("更新 {} 下载完成，等待安装", metadata.version_name);
    download_lease.complete(package_path);
    Ok(update)
}

fn proxy_mode_label(mode: UpdateProxyMode) -> &'static str {
    match mode {
        UpdateProxyMode::None => "不使用代理",
        UpdateProxyMode::System => "使用系统代理",
        UpdateProxyMode::Custom => "使用自定义代理",
    }
}

/// 取消当前文件下载。
#[tauri::command]
pub fn cancel_download(manager: tauri::State<'_, UpdateManager>) -> Result<(), String> {
    manager.cancel_download().map_err(|cancel_error| {
        warn!(
            operation = "cancel_download",
            error = %cancel_error,
            "取消更新下载请求被状态机拒绝"
        );
        cancel_error.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::{UpdateAvailability, UpdateInfo};

    #[test]
    fn update_availability_has_a_tagged_camel_case_contract() {
        assert_eq!(
            serde_json::to_value(UpdateAvailability::UpToDate).unwrap(),
            serde_json::json!({ "status": "upToDate" })
        );
        assert_eq!(
            serde_json::to_value(UpdateAvailability::Available {
                update: UpdateInfo {
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
}
