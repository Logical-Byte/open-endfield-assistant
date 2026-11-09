//! 统一二进制的 helper 模式入口、日志设施和原生提示适配。
use crate::{
    locale::UiLocale,
    platform::update::{UpdatePrompt, show_update_error},
    settings,
};
use oea_logger as logger;
use oea_update::install::{self, HelperHost, HelperLaunch, HelperLocale, InstallTarget};
use std::path::Path;

const HELPER_ARGUMENT: &str = "--oea-update-helper";

/// 当前 OEA 二进制承载 helper 入口，事务参数由内核生成。
pub(super) fn helper_launch(
    target: &InstallTarget,
    locale: UiLocale,
) -> Result<HelperLaunch, String> {
    let source_executable =
        std::env::current_exe().map_err(|error| format!("获取当前可执行文件路径失败: {error}"))?;
    Ok(HelperLaunch::new(
        source_executable,
        [HELPER_ARGUMENT.into()],
        target,
        locale.into(),
    ))
}

/// 普通启动时返回，识别到 helper 入口时由内核执行并退出进程。
pub fn run_helper_if_requested() {
    let mut args = std::env::args_os().skip(1);
    if args.any(|argument| argument == HELPER_ARGUMENT) {
        install::run_helper(args, settings::system_ui_locale().into(), OeaHelperHost);
    }
}

struct OeaHelperHost;

impl HelperHost for OeaHelperHost {
    type LogGuard = logger::LogGuard;
    type Prompt = UpdatePrompt;

    fn init_logging(&mut self, root: &Path) -> Self::LogGuard {
        let (guard, log_rx) = logger::init(&root.join("logs"));
        // helper 不转发日志条目，释放接收器以免无界队列积累日志。
        drop(log_rx);
        guard
    }

    fn show_progress(&mut self, locale: HelperLocale) -> Self::Prompt {
        let (title, progress, _) = helper_messages(locale.into());
        UpdatePrompt::new(title, progress)
    }

    fn show_success(&mut self, mut prompt: Self::Prompt, locale: HelperLocale) {
        let (title, _, success) = helper_messages(locale.into());
        prompt.show_success(title, success);
    }

    fn show_error(&mut self, locale: HelperLocale, error: &str) {
        show_helper_error(locale.into(), error);
    }
}

impl From<UiLocale> for HelperLocale {
    fn from(locale: UiLocale) -> Self {
        match locale {
            UiLocale::ZhCn => Self::ZhCn,
            UiLocale::EnUs => Self::EnUs,
        }
    }
}

impl From<HelperLocale> for UiLocale {
    fn from(locale: HelperLocale) -> Self {
        match locale {
            HelperLocale::ZhCn => Self::ZhCn,
            HelperLocale::EnUs => Self::EnUs,
        }
    }
}

fn helper_messages(locale: UiLocale) -> (&'static str, &'static str, &'static str) {
    match locale {
        UiLocale::ZhCn => (
            "OEA 更新",
            "正在提交程序更新，请稍候…",
            "程序更新已准备好，请重新启动 OEA 以完成更新",
        ),
        UiLocale::EnUs => (
            "OEA update",
            "Applying the application update. Please wait…",
            "The application update is ready. Restart OEA to complete the update.",
        ),
    }
}

fn show_helper_error(locale: UiLocale, error: &str) {
    let (title, prefix) = match locale {
        UiLocale::ZhCn => ("OEA 更新失败", "无法完成程序更新。错误详情："),
        UiLocale::EnUs => (
            "OEA update failed",
            "The application update could not be completed. Error details:",
        ),
    };
    show_update_error(title, &format!("{prefix}\n\n{error}"));
}
