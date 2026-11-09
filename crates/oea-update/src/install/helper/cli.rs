//! helper 模式内的参数协议和进程生命周期。模式入口由宿主识别并剥离。
use super::{HelperLaunch, HelperResult, InstallTarget, validate_helper_request};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use tracing::{debug, error, info};

const ROOT_ARGUMENT: &str = "--oea-update-root";
const EXECUTABLE_NAME_ARGUMENT: &str = "--oea-update-executable-name";
const UI_LOCALE_ARGUMENT: &str = "--oea-update-ui-locale";

/// helper 原生提示使用的语言，与宿主的设置类型独立。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperLocale {
    ZhCn,
    EnUs,
}

impl HelperLaunch {
    /// 使用显式源程序和宿主入口参数构造启动配置。
    ///
    /// `entry_args` 可为模式标志或子命令，原样放在事务参数前。
    /// 宿主须剥离这些入口参数，再把剩余参数交给 [`run_helper`]。
    /// 根目录和 executable 名称保持原生字符串，不要求 UTF-8。
    pub fn new(
        source_executable: impl Into<PathBuf>,
        entry_args: impl IntoIterator<Item = OsString>,
        target: &InstallTarget,
        locale: HelperLocale,
    ) -> Self {
        let mut args: Vec<_> = entry_args.into_iter().collect();
        args.extend([
            ROOT_ARGUMENT.into(),
            target.root().as_os_str().to_owned(),
            EXECUTABLE_NAME_ARGUMENT.into(),
            target.executable_name().clone(),
            UI_LOCALE_ARGUMENT.into(),
            match locale {
                HelperLocale::ZhCn => "zh-CN",
                HelperLocale::EnUs => "en-US",
            }
            .into(),
        ]);
        Self {
            source_executable: source_executable.into(),
            args,
        }
    }
}

/// helper 执行期间使用的宿主日志与原生提示。
///
/// 内核负责调用顺序。参数或运行身份无效时只调用 `show_error`，不初始化文件日志。
pub trait HelperHost {
    /// 日志保活资源，在结果提示完成后、进程退出前释放。
    type LogGuard;
    /// 进度提示资源。失败时先释放，再调用 `show_error`。
    type Prompt;

    /// `root` 已经过规范化和 helper 身份校验，可据此初始化文件日志。
    fn init_logging(&mut self, root: &Path) -> Self::LogGuard;
    /// 即将尝试提交 executable，此时事务已通过校验并持有锁。
    fn show_progress(&mut self, locale: HelperLocale) -> Self::Prompt;
    /// 消费进度提示并报告本次 executable 提交成功。
    fn show_success(&mut self, prompt: Self::Prompt, locale: HelperLocale);
    /// 报告参数、身份校验或事务执行错误。
    fn show_error(&mut self, locale: HelperLocale, error: &str);
}

/// 执行 helper 模式并退出进程，成功为 `0`，失败为 `1`。
///
/// `args` 仅包含模式入口之后的参数，不包含程序名或宿主的模式标志。
/// 内核自行获取当前程序路径并校验其位于目标的 `cache/update/helper[.exe]`。
/// 语言参数缺失或无效时使用 `default_locale`，事务参数非法时报告错误并退出。
/// 日志和提示资源以及 `host` 均在退出前释放。应在宿主创建其他保活资源前调用，
/// 因为进程退出不会运行调用栈上其他资源的析构函数。
pub fn run_helper(
    args: impl IntoIterator<Item = OsString>,
    default_locale: HelperLocale,
    mut host: impl HelperHost,
) -> ! {
    let exit_code = helper_exit_code(args, default_locale, &mut host);
    drop(host);
    std::process::exit(exit_code);
}

fn parse_arguments(
    args: impl IntoIterator<Item = OsString>,
    default_locale: HelperLocale,
) -> (Result<InstallTarget, String>, HelperLocale) {
    let mut args = args.into_iter().peekable();
    let mut root = None;
    let mut executable_name = None;
    let mut locale = None;
    while let Some(argument) = args.next() {
        if argument == ROOT_ARGUMENT {
            root = args.next().map(PathBuf::from);
        } else if argument == EXECUTABLE_NAME_ARGUMENT {
            executable_name = args.next();
        } else if argument == UI_LOCALE_ARGUMENT {
            // 可选语言缺少值时不能吞掉后续事务参数。
            if args.peek().is_some_and(|value| {
                value != ROOT_ARGUMENT
                    && value != EXECUTABLE_NAME_ARGUMENT
                    && value != UI_LOCALE_ARGUMENT
            }) {
                locale = args.next().and_then(|value| match value.to_str() {
                    Some("zh-CN") => Some(HelperLocale::ZhCn),
                    Some("en-US") => Some(HelperLocale::EnUs),
                    _ => None,
                });
            }
        }
    }
    let target = root
        .zip(executable_name)
        .ok_or_else(|| "helper 缺少应用根目录或 executable 名称参数".to_string())
        .and_then(|(root, name)| InstallTarget::new(root, name));
    (target, locale.unwrap_or(default_locale))
}

fn helper_exit_code(
    args: impl IntoIterator<Item = OsString>,
    default_locale: HelperLocale,
    host: &mut impl HelperHost,
) -> i32 {
    let (target, locale) = parse_arguments(args, default_locale);
    let request = target.and_then(|target| {
        let actual = std::env::current_exe()
            .map_err(|error| format!("获取 helper 实际路径失败: {error}"))?;
        validate_helper_request(
            target.root().to_owned(),
            target.executable_name().clone(),
            &actual,
        )
    });
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            host.show_error(locale, &error);
            eprintln!("更新 helper 失败: {error}");
            return 1;
        }
    };
    let _logger_guard = host.init_logging(request.root());
    debug!(
        process_role = "update_helper",
        pid = std::process::id(),
        root = %request.root().display(),
        "更新 helper 进程已启动"
    );
    info!("更新安装程序已启动");

    let mut prompt = None;
    let result = request.run(|| {
        prompt = Some(host.show_progress(locale));
    });
    if result == Ok(HelperResult::ExecutableCommitted) {
        host.show_success(
            prompt.take().expect("提交 executable 前必须创建提示"),
            locale,
        );
    }
    drop(prompt);
    match result {
        Ok(helper_result) => {
            debug!(
                process_role = "update_helper",
                result = ?helper_result,
                "更新 helper 进程执行完成"
            );
            match helper_result {
                HelperResult::NoTransaction => info!("没有需要安装的更新事务"),
                HelperResult::AlreadyCommitted => info!("程序更新已由先前的安装操作完成"),
                HelperResult::ExecutableCommitted => info!("程序更新已准备完成，请重新启动应用"),
            }
            0
        }
        Err(helper_error) => {
            error!(process_role = "update_helper", error = %helper_error, "更新 helper 进程执行失败");
            host.show_error(locale, &helper_error);
            eprintln!("更新 helper 失败: {helper_error}");
            1
        }
    }
}
