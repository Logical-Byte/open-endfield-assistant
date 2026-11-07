//! 更新安装的 OEA 适配：事件、原生提示、helper 参数及退出。
use crate::{
    app_paths::AppPaths,
    controller::Controller,
    update::{self, Updater},
};
use oea_update::install::InstallTarget;
use serde::Serialize;
use std::{
    ffi::OsString,
    sync::{Mutex, OnceLock},
};
use tauri::{Emitter, Manager};
use tracing::warn;
use ts_rs::TS;

pub(crate) mod extra;
mod helper;
mod startup;
pub use helper::{helper_request_from_args, run_helper_request, run_helper_request_with_logging};
pub(crate) use startup::initialize_at_startup;

/// 前端消费启动更新结果时，只区分是否有一次已完成的事务。
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "update/")]
pub enum StartupUpdateResult {
    NoTransaction,
    Completed,
}

static STARTUP_UPDATE_RESULT: OnceLock<Mutex<Option<StartupUpdateResult>>> = OnceLock::new();

fn startup_result_slot() -> &'static Mutex<Option<StartupUpdateResult>> {
    STARTUP_UPDATE_RESULT.get_or_init(|| Mutex::new(None))
}

/// 记录应用启动早期完成的资源事务结果，供 Tauri 前端在初始化后消费。
pub(crate) fn record_startup_update_result(result: oea_update::install::StartupUpdateResult) {
    if result == oea_update::install::StartupUpdateResult::Completed {
        *startup_result_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(StartupUpdateResult::Completed);
    }
}

/// 取出并清除本进程启动早期的更新结果，没有待消费结果时返回 `NoTransaction`。
#[tauri::command]
pub fn consume_startup_update_result() -> StartupUpdateResult {
    startup_result_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
        .unwrap_or(StartupUpdateResult::NoTransaction)
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "update/")]
enum InstallStage {
    Preparing,
    Extracting,
    ApplyingIncremental,
    ApplyingFull,
    CleaningUp,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export, export_to = "update/")]
struct InstallStageEvent {
    stage: InstallStage,
}

fn emit_install_stage(app: &tauri::AppHandle, stage: oea_update::install::InstallStage) {
    if let Err(emit_error) = app.emit(
        "update-install-stage",
        InstallStageEvent {
            stage: stage.into(),
        },
    ) {
        warn!(
            operation = "install",
            stage = ?stage,
            error = %emit_error,
            "向前端发送更新安装阶段失败，后端继续执行安装"
        );
    }
}

impl From<oea_update::install::InstallStage> for InstallStage {
    fn from(stage: oea_update::install::InstallStage) -> Self {
        match stage {
            oea_update::install::InstallStage::Preparing => Self::Preparing,
            oea_update::install::InstallStage::Extracting => Self::Extracting,
            oea_update::install::InstallStage::ApplyingIncremental => Self::ApplyingIncremental,
            oea_update::install::InstallStage::ApplyingFull => Self::ApplyingFull,
            oea_update::install::InstallStage::CleaningUp => Self::CleaningUp,
        }
    }
}

fn install_target(paths: &AppPaths) -> Result<InstallTarget, String> {
    let executable_name = if cfg!(target_os = "windows") {
        OsString::from("OEA.exe")
    } else {
        std::env::current_exe()
            .map_err(|error| format!("无法获取当前 exe 路径: {error}"))?
            .file_name()
            .map(OsString::from)
            .ok_or_else(|| "当前 exe 没有文件名".to_string())?
    };
    InstallTarget::new(paths.root_dir(), executable_name)
}

/// helper 启动成功后，请求当前应用通过原有生命周期退出。
#[tauri::command]
pub fn install_update(
    manager: tauri::State<'_, Updater>,
    app: tauri::AppHandle,
) -> Result<(), update::Error> {
    update::ensure_enabled()?;
    let target = install_target(&AppPaths::new()?)?;
    manager.install(
        &target,
        || {
            let locale = app.state::<Controller>().settings_snapshot().ui_locale;
            helper::helper_launch(&target, locale)
        },
        |stage| emit_install_stage(&app, stage),
    )?;
    app.exit(0);
    Ok(())
}
