//! 在创建窗口和加载资源前静默完成更新事务。
use crate::app_paths::AppPaths;
use oea_update::install;
use tracing::error;

pub(crate) fn initialize_at_startup(paths: &AppPaths) -> anyhow::Result<()> {
    let target = super::install_target(paths).map_err(anyhow::Error::msg)?;
    install::complete_startup_transaction(&target, || {}).map_err(|error| {
        error!(operation = "startup_transaction", error = %error, "启动时完成更新事务失败");
        anyhow::anyhow!("启动时完成更新事务失败: {error}")
    })?;
    Ok(())
}
