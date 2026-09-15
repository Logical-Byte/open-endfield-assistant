//! v2 启动期的事务 adapter。

use serde::Serialize;

use super::{transaction, workspace::InstallTarget};

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
    transaction::complete_startup(target)
}
