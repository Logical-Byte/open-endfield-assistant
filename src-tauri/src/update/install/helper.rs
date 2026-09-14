//! helper 子进程的参数、启动和 exe 提交。
//!
//! helper 与 v1/v2 通过 `transaction.lock` 串行访问事务。helper 在 v1 退出前启动，
//! 会重试原子替换 exe，直到 v1 释放文件；替换成功只会让 candidate exe 消失，资源
//! 仍交给新 exe 的启动阶段提交。替换失败则删除整个事务 workspace，下一次必须重新
//! 下载并构造 candidate。

use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};
use tracing::{debug, error, info};

use crate::{
    app_paths::AppPaths,
    platform::update::{UpdatePrompt, replace_file, show_update_error},
};

use super::workspace::UpdateWorkspace;

/// helper 模式命令行参数。
pub(super) const HELPER_ARGUMENT: &str = "--oea-update-helper";
/// helper 根目录参数。
pub(super) const ROOT_ARGUMENT: &str = "--oea-update-root";
/// helper 目标应用 executable name 参数。
pub(super) const EXECUTABLE_NAME_ARGUMENT: &str = "--oea-update-executable-name";
const REPLACE_RETRY_WINDOW: Duration = Duration::from_secs(30);
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(100);

/// helper 执行结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperResult {
    /// 未发现事务。
    NoTransaction,
    /// candidate exe 已经消失，说明 helper 之前已完成 exe 提交。
    AlreadyCommitted,
    /// 本次 helper 成功提交了 exe。
    ExecutableCommitted,
}

/// 从命令行参数解析 helper 的根目录和目标 executable name。
pub fn helper_request_from_args<I>(args: I) -> Option<(PathBuf, OsString)>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut root = None;
    let mut executable_name = None;
    while let Some(argument) = args.next() {
        if argument == HELPER_ARGUMENT {
            while let Some(argument) = args.next() {
                if argument == ROOT_ARGUMENT {
                    root = args.next().map(PathBuf::from);
                } else if argument == EXECUTABLE_NAME_ARGUMENT {
                    executable_name = args.next();
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
            return Some((root, executable_name));
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
) -> Result<HelperResult, String> {
    let result =
        validate_helper_request(root, executable_name).and_then(|workspace| run_helper(&workspace));
    if let Err(error) = &result {
        show_update_error("OEA 更新失败", error);
    }
    result
}

/// 校验 helper 参数后启用应用文件日志并执行事务。
///
/// 日志目录只能从已经验证过的应用根目录派生，避免外部参数让 helper 在任意位置写文件。
pub fn run_helper_request_with_logging(
    root: PathBuf,
    executable_name: OsString,
) -> Result<HelperResult, String> {
    let workspace = match validate_helper_request(root, executable_name) {
        Ok(workspace) => workspace,
        Err(error) => {
            show_update_error("OEA 更新失败", &error);
            return Err(error);
        }
    };
    let (_logger_guard, _log_rx) = crate::logger::init(&workspace.app_paths().logs_dir());
    debug!(
        process_role = "update_helper",
        pid = std::process::id(),
        root = %workspace.root().display(),
        "更新 helper 进程已启动"
    );
    info!("更新安装程序已启动");

    let result = run_helper(&workspace);
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
        show_update_error("OEA 更新失败", helper_error);
    }
    result
}

fn validate_helper_request(
    root: PathBuf,
    executable_name: OsString,
) -> Result<UpdateWorkspace, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("解析 helper 应用根目录失败: {error}"))?;
    let app_paths = AppPaths::with_root_dir(root);
    let workspace = UpdateWorkspace::with_executable_name(&app_paths, executable_name);
    let actual_helper = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("解析 helper 实际路径失败: {error}"))?;
    let expected_helper = workspace
        .helper_path()
        .canonicalize()
        .map_err(|error| format!("解析 helper 期望路径失败: {error}"))?;
    if actual_helper != expected_helper {
        return Err(format!(
            "拒绝从更新工作区外执行 helper: {}",
            actual_helper.display()
        ));
    }
    Ok(workspace)
}

/// 启动真实的 helper 子进程。当前应用退出后，helper 会从相同的根目录继续事务。
pub(super) fn spawn_helper(workspace: &UpdateWorkspace) -> Result<Child, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("获取当前可执行文件路径失败: {error}"))?;
    spawn_helper_with_executable(workspace, &executable)
}

/// 复制显式 source binary 到 cache/update/helper，再从副本启动 helper。
///
/// 不能直接执行根目录下当前 exe：Windows 可能仍持有它，且 helper 随后要原子替换
/// 这个路径。source 复制失败或子进程启动失败时会清理 transaction workspace；helper
/// 副本本身留给后续启动的 best-effort cleanup，因为当前进程可能仍在使用它。
pub(super) fn spawn_helper_with_executable(
    workspace: &UpdateWorkspace,
    source_executable: &Path,
) -> Result<Child, String> {
    let helper_executable = match workspace.prepare_helper_copy(source_executable) {
        Ok(path) => path,
        Err(error) => {
            return Err(with_cleanup_error(
                workspace,
                format!("准备更新 helper 失败: {error}"),
            ));
        }
    };
    let result = Command::new(&helper_executable)
        .arg(HELPER_ARGUMENT)
        .arg(ROOT_ARGUMENT)
        .arg(workspace.root())
        .arg(EXECUTABLE_NAME_ARGUMENT)
        .arg(workspace.executable_name())
        .spawn();
    match result {
        Ok(child) => Ok(child),
        Err(error) => Err(with_cleanup_error(
            workspace,
            format!("启动更新 helper 失败: {error}"),
        )),
    }
}

/// 执行一次 helper 的 exe 提交。
///
/// 成功前 candidate exe 和根 exe 都保持原位；成功后 candidate exe 已经原子地移动到
/// 根路径，`transaction.json` 保持不变，candidate/resources 留待 v2 启动提交。若在
/// 重试窗口内仍无法替换，事务材料会被删除，根目录保持旧版本，调用方下次必须重新
/// 下载 package。
pub fn run_helper(workspace: &UpdateWorkspace) -> Result<HelperResult, String> {
    debug!(
        process_role = "update_helper",
        lock_path = %workspace.lock_path().display(),
        "更新 helper 正在等待事务锁"
    );
    let _lock = workspace
        .lock()
        .map_err(|error| format!("获取 transaction.lock 失败: {error}"))?;
    debug!(process_role = "update_helper", "更新 helper 已取得事务锁");

    if workspace.read_transaction()?.is_none() {
        debug!(
            process_role = "update_helper",
            result = "no_transaction",
            "更新 helper 未发现待处理事务"
        );
        return Ok(HelperResult::NoTransaction);
    }

    let candidate_executable = workspace.candidate_path().join(workspace.executable_name());
    if !candidate_executable.exists() {
        debug!(
            process_role = "update_helper",
            result = "already_committed",
            "更新 helper 确认 executable 已由先前操作提交"
        );
        return Ok(HelperResult::AlreadyCommitted);
    }

    let mut prompt = UpdatePrompt::new("OEA 更新", "正在提交程序更新，请稍候…");

    let target_executable = workspace.executable_path();
    let started_at = Instant::now();
    let deadline = started_at + REPLACE_RETRY_WINDOW;
    let mut attempts = 0u32;
    debug!(
        process_role = "update_helper",
        candidate = %candidate_executable.display(),
        target = %target_executable.display(),
        "更新 helper 开始提交 executable"
    );
    let last_error = loop {
        attempts += 1;
        match replace_file(&candidate_executable, &target_executable) {
            Ok(()) => {
                debug!(
                    process_role = "update_helper",
                    attempts,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "更新 helper 已提交 executable"
                );
                prompt.show_success("OEA 更新", "程序更新已准备好，请重新启动 OEA 以完成更新");
                return Ok(HelperResult::ExecutableCommitted);
            }
            Err(replace_error) if Instant::now() < deadline => {
                if attempts == 1 {
                    debug!(
                        process_role = "update_helper",
                        error = %replace_error,
                        "executable 暂时无法替换，等待旧进程释放文件"
                    );
                }
                thread::sleep(REPLACE_RETRY_DELAY);
            }
            Err(error) => {
                break error;
            }
        }
    };

    let reason = last_error.to_string();
    Err(with_cleanup_error(
        workspace,
        format!("替换应用 exe 失败: {reason}"),
    ))
}

fn with_cleanup_error(workspace: &UpdateWorkspace, message: String) -> String {
    match workspace.remove_transaction_workspace() {
        Ok(()) => message,
        Err(cleanup_error) => format!("{message}；清理事务也失败: {cleanup_error}"),
    }
}
