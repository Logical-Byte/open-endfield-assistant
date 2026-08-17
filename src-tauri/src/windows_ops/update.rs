//! 自动更新 Bootstrap 的原生平台实现。

use std::process::Command;

use anyhow::Result;

use crate::update::bootstrap::{BootstrapInput, PlatformOps};

use super::details;

/// 把平台无关 Bootstrap 流程委托给当前操作系统 API 的零状态适配器。
pub struct NativePlatform;

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
