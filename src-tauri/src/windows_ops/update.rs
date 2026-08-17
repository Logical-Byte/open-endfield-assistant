//! 自动更新 Bootstrap 的平台操作接口。

#[cfg(target_os = "windows")]
use std::process::Command;

use anyhow::Result;
#[cfg(target_os = "macos")]
use anyhow::bail;

use crate::update::bootstrap::{BootstrapInput, PlatformOps};

#[cfg(target_os = "windows")]
use super::details;

pub struct NativePlatform;

#[cfg(target_os = "windows")]
impl PlatformOps for NativePlatform {
    fn wait_for_process(&self, pid: u32) -> Result<()> {
        details::update::wait_for_process(pid)
    }

    fn replace_entrypoint(&self, input: &BootstrapInput, source_version: &str) -> Result<()> {
        details::update::replace_entrypoint(input, source_version)
    }

    fn launch_entrypoint(&self, input: &BootstrapInput) -> Result<()> {
        Command::new(input.portable_root().join("OEA.exe"))
            .current_dir(input.portable_root())
            .spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl PlatformOps for NativePlatform {
    fn wait_for_process(&self, _pid: u32) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }

    fn replace_entrypoint(&self, _input: &BootstrapInput, _source_version: &str) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }

    fn launch_entrypoint(&self, _input: &BootstrapInput) -> Result<()> {
        bail!("Bootstrap update is not supported on macOS")
    }
}
