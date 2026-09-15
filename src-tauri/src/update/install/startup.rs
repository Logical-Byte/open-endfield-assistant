//! v2 启动期的事务 adapter。

use serde::Serialize;

use crate::platform::update::UpdatePrompt;
use tracing::info;

use super::{
    transaction,
    workspace::{InstallTarget, InstallWorkspace},
};

/// 启动阶段对更新事务的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupUpdateResult {
    NoTransaction,
    WaitingForHelper,
    Completed,
}

/// 尝试在应用初始化前完成资源事务。
pub(crate) fn complete_startup_transaction(
    target: &InstallTarget,
) -> Result<StartupUpdateResult, String> {
    let mut prompt = None;
    let result = transaction::complete_startup(target, || {
        prompt = Some(UpdatePrompt::new("OEA 更新", "正在完成资源更新，请稍候…"));
    });
    match result {
        Ok(StartupUpdateResult::NoTransaction) => {
            InstallWorkspace::new(target).best_effort_cleanup();
            Ok(StartupUpdateResult::NoTransaction)
        }
        Ok(StartupUpdateResult::Completed) => {
            InstallWorkspace::new(target).best_effort_cleanup();
            prompt.expect("提交 resources 前必须创建提示").finish();
            info!("更新安装完成");
            Ok(StartupUpdateResult::Completed)
        }
        other => other,
    }
}
