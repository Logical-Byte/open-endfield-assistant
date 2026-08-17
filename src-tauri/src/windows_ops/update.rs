//! 自动更新 Bootstrap 的原生平台实现。

#[cfg(target_os = "windows")]
use std::process::Command;

use anyhow::Result;
#[cfg(target_os = "macos")]
use anyhow::bail;

use crate::update::bootstrap::{BootstrapInput, PlatformOps};

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
    /// macOS 仅提供开发外壳，不执行正式版 Bootstrap。
    fn wait_for_process(&self, _pid: u32) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }

    /// macOS 开发外壳明确拒绝替换应用入口。
    fn replace_entrypoint(&self, _input: &BootstrapInput, _source_version: &str) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }

    /// macOS 开发外壳明确拒绝启动 Windows 便携入口。
    fn launch_entrypoint(&self, _input: &BootstrapInput) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }
}
