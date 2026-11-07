//! 保留旧 WebView 入口的传输 adapter，业务流程与 GPUI 共用。
use crate::{
    controller::Controller,
    update::{self, UpdateManager, commands, install},
};
use std::sync::Arc;
use tauri::{Emitter, Manager};

#[tauri::command]
pub fn get_update_status(
    manager: tauri::State<'_, UpdateManager>,
) -> update::manager::UpdateStatus {
    manager.status()
}

#[tauri::command]
pub async fn check_update(
    manager: tauri::State<'_, UpdateManager>,
    controller: tauri::State<'_, Controller>,
    app: tauri::AppHandle,
) -> Result<commands::UpdateAvailability, update::Error> {
    commands::check_update(
        &manager,
        &controller,
        &app.package_info().version.to_string(),
    )
    .await
}

#[tauri::command]
pub async fn download_update(
    manager: tauri::State<'_, UpdateManager>,
    controller: tauri::State<'_, Controller>,
    app: tauri::AppHandle,
    on_progress: tauri::ipc::Channel<commands::DownloadProgress>,
) -> Result<commands::DownloadOutcome, update::Error> {
    commands::download_update(
        &manager,
        &controller,
        &app.package_info().version.to_string(),
        Arc::new(move |progress| {
            on_progress
                .send(progress)
                .map_err(|error| error.to_string())
        }),
    )
    .await
}

#[tauri::command]
pub fn cancel_download(manager: tauri::State<'_, UpdateManager>) -> Result<(), update::Error> {
    commands::cancel_download(&manager)
}

#[tauri::command]
pub fn consume_startup_update_result() -> install::StartupUpdateResult {
    install::consume_startup_update_result()
}

#[tauri::command]
pub fn install_update(
    manager: tauri::State<'_, UpdateManager>,
    app: tauri::AppHandle,
) -> Result<(), update::Error> {
    let locale = app.state::<Controller>().settings_snapshot().ui_locale;
    install::install_update(&manager, locale, |stage| {
        let _ = app.emit("update-install-stage", install::InstallStageEvent { stage });
    })?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn developer_install_update(
    manager: tauri::State<'_, UpdateManager>,
    app: tauri::AppHandle,
) -> Result<bool, update::Error> {
    let locale = app.state::<Controller>().settings_snapshot().ui_locale;
    let installed = install::extra::developer_install_update(&manager, locale, |stage| {
        let _ = app.emit("update-install-stage", install::InstallStageEvent { stage });
    })?;
    if installed {
        app.exit(0);
    }
    Ok(installed)
}
