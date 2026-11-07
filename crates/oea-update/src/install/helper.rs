//! helper 子进程的启动和 exe 提交。
//!
//! helper 与 v1/v2 通过 `transaction.lock` 串行访问事务。helper 在 v1 退出前启动，
//! 会重试原子替换 exe，直到 v1 释放文件；替换成功只会让 candidate exe 消失，资源
//! 仍交给新 exe 的启动阶段提交。替换失败则删除整个事务 workspace，下一次必须重新
//! 下载并构造 candidate。
use super::{transaction, workspace::InstallTarget};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Child, Command},
};

/// 宿主选择的 helper 源程序与原生参数，内核只复制并启动。
#[derive(Debug, Clone)]
pub struct HelperLaunch {
    pub source_executable: PathBuf,
    pub args: Vec<OsString>,
}

/// 仅表示 helper 进程已启动。宿主仍须请求退出以释放目标 executable。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelperStarted {
    pub process_id: u32,
}

/// 已校验运行位置的 helper 请求，供宿主在初始化日志后执行。
pub struct HelperRequest {
    target: InstallTarget,
}
impl HelperRequest {
    pub fn root(&self) -> &Path {
        self.target.root()
    }
    pub fn run(&self, on_commit: impl FnOnce()) -> Result<HelperResult, String> {
        transaction::commit_executable(&self.target, on_commit)
    }
}

/// 校验实际 helper 路径。宿主须传入实际运行程序的路径。
pub fn validate_helper_request(
    root: PathBuf,
    executable_name: OsString,
    actual_helper: &Path,
) -> Result<HelperRequest, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("解析 helper 应用根目录失败: {error}"))?;
    let target = InstallTarget::new(root, executable_name)?;
    target.validate_helper_executable(actual_helper)?;
    Ok(HelperRequest { target })
}

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

/// 复制显式 source binary 到 cache/update/helper，再从副本启动 helper。
///
/// 不能直接执行根目录下当前 exe：Windows 可能仍持有它，且 helper 随后要原子替换
/// 这个路径。source 复制失败或子进程启动失败时会清理 transaction workspace；helper
/// 副本本身留给后续启动的 best-effort cleanup，因为当前进程可能仍在使用它。
pub(super) fn spawn_helper(target: &InstallTarget, launch: &HelperLaunch) -> Result<Child, String> {
    let helper_executable = target
        .prepare_helper_copy(&launch.source_executable)
        .map_err(|error| format!("准备更新 helper 失败: {error}"))?;
    let result = Command::new(&helper_executable).args(&launch.args).spawn();
    match result {
        Ok(child) => Ok(child),
        Err(error) => Err(format!("启动更新 helper 失败: {error}")),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    };

    #[test]
    fn launches_the_explicit_source_with_native_host_arguments() {
        let root = tempfile::tempdir().unwrap();
        let target = InstallTarget::new(root.path(), "OEA").unwrap();
        let source = root.path().join("host-helper");
        let output = root.path().join("argument output");
        fs::write(&source, "#!/bin/sh\nprintf '%s' \"$1\" > \"$2\"\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let argument = OsString::from_vec(b"host argument \xff".to_vec());
        let launch = HelperLaunch {
            source_executable: source,
            args: vec![argument.clone(), output.into_os_string()],
        };
        let mut helper = spawn_helper(&target, &launch).unwrap();
        assert!(helper.wait().unwrap().success());
        assert_eq!(
            fs::read(root.path().join("argument output")).unwrap(),
            argument.into_vec()
        );
    }
}
