//! 统一二进制的 helper 参数分派、日志和原生提示。
use crate::{
    locale::UiLocale,
    platform::update::{UpdatePrompt, show_update_error},
    settings,
};
use oea_logger as logger;
use oea_update::install::{self, HelperLaunch, HelperRequest, HelperResult, InstallTarget};
use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};
use tracing::{debug, error, info};

/// helper 模式命令行参数。
pub(super) const HELPER_ARGUMENT: &str = "--oea-update-helper";
/// helper 根目录参数。
pub(super) const ROOT_ARGUMENT: &str = "--oea-update-root";
/// helper 目标应用 executable name 参数。
pub(super) const EXECUTABLE_NAME_ARGUMENT: &str = "--oea-update-executable-name";
const UI_LOCALE_ARGUMENT: &str = "--oea-update-ui-locale";

/// 当前 OEA 二进制承载 helper 入口，参数由 OEA 显式交给内核。
pub(super) fn helper_launch(
    target: &InstallTarget,
    locale: UiLocale,
) -> Result<HelperLaunch, String> {
    let source_executable =
        std::env::current_exe().map_err(|error| format!("获取当前可执行文件路径失败: {error}"))?;
    Ok(HelperLaunch {
        source_executable,
        args: vec![
            HELPER_ARGUMENT.into(),
            ROOT_ARGUMENT.into(),
            target.root().as_os_str().to_owned(),
            EXECUTABLE_NAME_ARGUMENT.into(),
            target.executable_name().clone(),
            UI_LOCALE_ARGUMENT.into(),
            match locale {
                UiLocale::ZhCn => "zh-CN",
                UiLocale::EnUs => "en-US",
            }
            .into(),
        ],
    })
}

/// 从命令行参数解析事务目标和提示语言快照。
/// 语言参数缺失或无效时使用系统语言，不影响合法事务参数。
pub fn helper_request_from_args<I>(args: I) -> Option<(PathBuf, OsString, UiLocale)>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter().peekable();
    let mut root = None;
    let mut executable_name = None;
    let mut locale = None;
    while let Some(argument) = args.next() {
        if argument == HELPER_ARGUMENT {
            while let Some(argument) = args.next() {
                if argument == ROOT_ARGUMENT {
                    root = args.next().map(PathBuf::from);
                } else if argument == EXECUTABLE_NAME_ARGUMENT {
                    executable_name = args.next();
                } else if argument == UI_LOCALE_ARGUMENT {
                    // 语言是可选提示参数。缺少值时不能吞掉后续事务参数。
                    if args.peek().is_some_and(|value| {
                        value != ROOT_ARGUMENT
                            && value != EXECUTABLE_NAME_ARGUMENT
                            && value != UI_LOCALE_ARGUMENT
                    }) {
                        locale = args.next().and_then(|value| match value.to_str() {
                            Some("zh-CN") => Some(UiLocale::ZhCn),
                            Some("en-US") => Some(UiLocale::EnUs),
                            _ => None,
                        });
                    }
                }
            }
            let (root, executable_name) = root.zip(executable_name)?;
            let mut components = Path::new(&executable_name).components();
            if !root.is_absolute()
                || !matches!(components.next(), Some(Component::Normal(_)))
                || components.next().is_some()
            {
                return None;
            }
            return Some((
                root,
                executable_name,
                locale.unwrap_or_else(settings::system_ui_locale),
            ));
        }
    }
    None
}

/// 校验 helper 的实际位置后执行事务。
///
/// 根目录来自命令行，是外部输入。helper 必须真实位于该根目录的
/// `cache/update/helper[.exe]`，随后所有事务路径才允许从这个根目录派生。根目录先
/// canonicalize，避免 `..` 或符号链接让 executable 目标逃出 helper 所属应用。
pub fn run_helper_request(
    root: PathBuf,
    executable_name: OsString,
    locale: UiLocale,
) -> Result<HelperResult, String> {
    let result = validate_helper_request(root, executable_name)
        .and_then(|request| run_helper(&request, locale));
    if let Err(error) = &result {
        show_helper_error(locale, error);
    }
    result
}

/// 校验 helper 参数后启用应用文件日志并执行事务。
///
/// 日志目录只能从已经验证过的应用根目录派生，避免外部参数让 helper 在任意位置写文件。
pub fn run_helper_request_with_logging(
    root: PathBuf,
    executable_name: OsString,
    locale: UiLocale,
) -> Result<HelperResult, String> {
    let request = match validate_helper_request(root, executable_name) {
        Ok(request) => request,
        Err(error) => {
            show_helper_error(locale, &error);
            return Err(error);
        }
    };
    let (_logger_guard, log_rx) = logger::init(&request.root().join("logs"));
    // helper 不转发日志条目，释放接收器以免无界队列积累日志。
    drop(log_rx);
    debug!(
        process_role = "update_helper",
        pid = std::process::id(),
        root = %request.root().display(),
        "更新 helper 进程已启动"
    );
    info!("更新安装程序已启动");

    let result = run_helper(&request, locale);
    match &result {
        Ok(helper_result) => {
            debug!(
                process_role = "update_helper",
                result = ?helper_result,
                "更新 helper 进程执行完成"
            );
            match helper_result {
                HelperResult::NoTransaction => info!("没有需要安装的更新事务"),
                HelperResult::AlreadyCommitted => info!("程序更新已由先前的安装操作完成"),
                HelperResult::ExecutableCommitted => {
                    info!("程序更新已准备完成，请重新启动 OEA")
                }
            }
        }
        Err(helper_error) => error!(
            process_role = "update_helper",
            error = %helper_error,
            "更新 helper 进程执行失败"
        ),
    }
    if let Err(helper_error) = &result {
        show_helper_error(locale, helper_error);
    }
    result
}

fn validate_helper_request(
    root: PathBuf,
    executable_name: OsString,
) -> Result<HelperRequest, String> {
    let actual_helper =
        std::env::current_exe().map_err(|error| format!("获取 helper 实际路径失败: {error}"))?;
    install::validate_helper_request(root, executable_name, &actual_helper)
}

/// 执行一次 helper 的 exe 提交。
///
/// 成功前 candidate exe 和根 exe 都保持原位；成功后 candidate exe 已经原子地移动到
/// 根路径，`transaction.json` 保持不变，candidate/resources 留待 v2 启动提交。若在
/// 重试窗口内仍无法替换，事务材料会被删除，根目录保持旧版本，调用方下次必须重新
/// 下载 package。
fn run_helper(request: &HelperRequest, locale: UiLocale) -> Result<HelperResult, String> {
    // 同一个 helper 从进度到结果始终使用启动时的语言快照。
    let (title, progress, success) = helper_messages(locale);
    let mut prompt = None;
    let result = request.run(|| {
        prompt = Some(UpdatePrompt::new(title, progress));
    });
    if result == Ok(HelperResult::ExecutableCommitted) {
        prompt
            .expect("提交 executable 前必须创建提示")
            .show_success(title, success);
    }
    result
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
