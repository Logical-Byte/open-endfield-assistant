//! v2 启动期的事务 adapter。

use crate::platform::update::UpdatePrompt;
use tracing::error;

use crate::{app_paths::AppPaths, locale::UiLocale, settings};

use oea_update::install::{self, InstallTarget, StartupUpdateResult};

/// 完成启动期更新并保存前端可消费的结果。
///
/// 调用方须先启用日志，并在本函数成功返回后才能创建窗口或加载模型、静态数据。
/// 更新目标解析、事务提交及结果记录由安装模块统一负责。
pub(crate) fn initialize_at_startup(paths: &AppPaths) -> anyhow::Result<()> {
    let target = super::install_target(paths)
        .map_err(|error| anyhow::anyhow!("无法确定更新 executable name: {error}"))?;
    let result = complete_startup_transaction(
        &target,
        settings::read_ui_locale(&paths.oea_settings_file()),
    )
    .map_err(|error| {
        error!(
            operation = "startup_transaction",
            error = %error,
            "启动时完成更新事务失败"
        );
        #[cfg(target_os = "macos")]
        {
            let locale = settings::read_ui_locale(&paths.oea_settings_file());
            let title = match locale {
                UiLocale::ZhCn => "OEA 更新失败",
                UiLocale::EnUs => "OEA update failed",
            };
            crate::platform::update::show_update_error(title, &error);
        }
        anyhow::anyhow!("启动时完成更新事务失败: {error}")
    })?;
    super::record_startup_update_result(result);
    Ok(())
}

/// 尝试在应用初始化前完成资源事务。
pub(crate) fn complete_startup_transaction(
    target: &InstallTarget,
    locale: UiLocale,
) -> Result<StartupUpdateResult, String> {
    let (title, progress) = startup_messages(locale);
    let mut prompt = None;
    let result = install::complete_startup_transaction(target, || {
        prompt = Some(UpdatePrompt::new(title, progress));
    });
    match result {
        Ok(StartupUpdateResult::NoTransaction) => Ok(StartupUpdateResult::NoTransaction),
        Ok(StartupUpdateResult::Completed) => {
            if let Some(mut prompt) = prompt {
                prompt.finish();
            }
            Ok(StartupUpdateResult::Completed)
        }
        other => other,
    }
}

fn startup_messages(locale: UiLocale) -> (&'static str, &'static str) {
    match locale {
        UiLocale::ZhCn => ("OEA 更新", "正在完成资源更新，请稍候…"),
        UiLocale::EnUs => ("OEA update", "Completing the resource update. Please wait…"),
    }
}
