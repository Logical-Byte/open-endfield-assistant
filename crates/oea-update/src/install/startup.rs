//! v2 启动期的事务恢复。
use super::{
    transaction,
    workspace::{InstallTarget, InstallWorkspace},
};
use tracing::info;

/// 启动阶段对更新事务的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupUpdateResult {
    NoTransaction,
    WaitingForHelper,
    Completed,
}

/// 尝试在应用初始化和资源消费者创建前完成资源事务。
/// `on_commit` 仅在开始移动资源时调用，宿主可在其中显示进度提示。
pub fn complete_startup_transaction(
    target: &InstallTarget,
    on_commit: impl FnOnce(),
) -> Result<StartupUpdateResult, String> {
    let result = transaction::complete_startup(target, on_commit);
    match result {
        Ok(StartupUpdateResult::NoTransaction) => {
            InstallWorkspace::new(target).best_effort_cleanup();
            Ok(StartupUpdateResult::NoTransaction)
        }
        Ok(StartupUpdateResult::Completed) => {
            InstallWorkspace::new(target).best_effort_cleanup();
            info!("更新安装完成");
            Ok(StartupUpdateResult::Completed)
        }
        other => other,
    }
}
