//! 自动更新 Bootstrap 的原生平台实现。

#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::{fs, io, thread, time::Duration};

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;

use crate::update::bootstrap::{BootstrapInput, PlatformOps};
#[cfg(target_os = "macos")]
use crate::update::transaction::CANDIDATE_EXE;

#[cfg(target_os = "windows")]
use super::details;

/// 把平台无关 Bootstrap 流程委托给当前操作系统 API 的零状态适配器。
pub struct NativePlatform;

#[cfg(target_os = "windows")]
impl PlatformOps for NativePlatform {
    /// 用 Win32 进程句柄等待旧程序退出；已经退出也视为成功。
    fn wait_for_process(&self, pid: u32) -> Result<()> {
        details::update::wait_for_process(pid)
    }

    /// 用 Win32 原子替换入口，并在便携根目录保留旧版本备份。
    fn replace_entrypoint(&self, input: &BootstrapInput, source_version: &str) -> Result<()> {
        details::update::replace_entrypoint(input, source_version)
    }

    /// 启动根目录中的新入口，并把工作目录设为便携根目录。
    fn launch_entrypoint(&self, input: &BootstrapInput) -> Result<()> {
        Command::new(input.portable_root().join("OEA.exe"))
            .current_dir(input.portable_root())
            .spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl PlatformOps for NativePlatform {
    /// 轮询调用方 PID，直到 macOS 报告该进程已经不存在。
    ///
    /// 这是开发测试实现；正式 Windows 实现使用可等待的进程句柄，不需要轮询。
    fn wait_for_process(&self, pid: u32) -> Result<()> {
        let pid = libc::pid_t::try_from(pid).context("调用方 PID 超出 macOS 支持范围")?;
        loop {
            let result = unsafe { libc::kill(pid, 0) };
            if result == 0 {
                thread::sleep(Duration::from_millis(25));
                continue;
            }

            let error = io::Error::last_os_error();
            match error.raw_os_error() {
                Some(libc::ESRCH) => return Ok(()),
                // `EPERM` 仍证明进程存在，只是当前用户不能向它发送信号。
                Some(libc::EPERM) => thread::sleep(Duration::from_millis(25)),
                _ => return Err(error).context("检查调用方进程状态失败"),
            }
        }
    }

    /// 为旧入口创建硬链接备份，再原子重命名候选入口覆盖 `OEA.exe`。
    ///
    /// 两个入口都位于同一个便携根目录文件系统中；`rename` 切换期间不会出现入口缺失。
    fn replace_entrypoint(&self, input: &BootstrapInput, source_version: &str) -> Result<()> {
        let entrypoint = input.portable_root().join("OEA.exe");
        let candidate = input.transaction_dir().join(CANDIDATE_EXE);
        let backup = input
            .portable_root()
            .join(format!("OEA-backup-v{source_version}.exe"));

        fs::hard_link(&entrypoint, &backup).context("备份旧 OEA.exe 失败")?;
        fs::rename(&candidate, &entrypoint).context("原子替换 OEA.exe 失败")?;
        Ok(())
    }

    /// 启动测试便携根目录中的替换入口；正式 Windows ZIP 仍不能在 macOS 运行。
    fn launch_entrypoint(&self, input: &BootstrapInput) -> Result<()> {
        Command::new(input.portable_root().join("OEA.exe"))
            .current_dir(input.portable_root())
            .spawn()?;
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::{fs, process::Command, thread};

    use crate::update::{
        bootstrap::{BootstrapInput, run},
        transaction::CANDIDATE_EXE,
    };

    use super::NativePlatform;

    #[test]
    fn bootstrap_waits_then_backs_up_replaces_and_launches() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("portable");
        let transaction = root.join("cache/updates/current");
        fs::create_dir_all(transaction.join("candidate")).unwrap();
        fs::write(root.join("OEA.exe"), b"old entrypoint").unwrap();
        fs::copy("/usr/bin/true", transaction.join(CANDIDATE_EXE)).unwrap();

        let mut caller = Command::new("/bin/sh")
            .args(["-c", "sleep 0.1"])
            .spawn()
            .unwrap();
        let caller_pid = caller.id();
        let reaper = thread::spawn(move || caller.wait().unwrap());
        let input = BootstrapInput::new(&root, &transaction).unwrap();

        run(&input, caller_pid, "0.1.0", &NativePlatform).unwrap();
        reaper.join().unwrap();

        assert_eq!(
            fs::read(root.join("OEA-backup-v0.1.0.exe")).unwrap(),
            b"old entrypoint"
        );
        assert_eq!(
            fs::read(root.join("OEA.exe")).unwrap(),
            fs::read("/usr/bin/true").unwrap()
        );
    }
}
