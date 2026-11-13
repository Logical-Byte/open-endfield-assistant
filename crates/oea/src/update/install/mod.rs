//! 更新安装的 OEA 适配：事件、原生提示、helper 模式入口及主进程退出。
use crate::{
    app_paths::AppPaths,
    controller::Controller,
    update::{self, Updater},
};
use oea_update::install::InstallTarget;
use serde::Serialize;
use std::ffi::OsString;
use tauri::{Emitter, Manager};
use tracing::warn;
use ts_rs::TS;

pub(crate) mod extra;
mod helper;
mod startup;
pub use helper::run_helper_if_requested;
pub(crate) use startup::initialize_at_startup;

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
    restart: bool,
) -> Result<(), update::Error> {
    update::ensure_enabled()?;
    let target = install_target(&AppPaths::new()?)?;
    app.state::<Controller>().install_when_idle(|| {
        manager
            .install(
                &target,
                || {
                    let locale = app.state::<Controller>().ui_locale();
                    helper::helper_launch(&target, locale, restart)
                },
                |stage| emit_install_stage(&app, stage),
            )
            .map_err(Into::into)
    })?;
    app.exit(0);
    Ok(())
}
