//! 自动更新：下载、进度、取消与安装状态。

use crate::app_variant::{self, AppVariant};

pub mod commands;
pub mod install;

mod check;
mod download;
pub mod error;
mod http;
mod manager;
mod response;
mod source;

pub use error::Error;
pub use manager::UpdateManager;

/// OEA dev 不参与自更新，避免更新流程改写工作区资源。
pub(crate) const fn allow_variant_update() -> bool {
    match app_variant::CURRENT {
        AppVariant::Dev => false,
        AppVariant::Release => true,
    }
}

fn ensure_enabled() -> Result<(), Error> {
    if allow_variant_update() {
        Ok(())
    } else {
        Err(Error::new(error::Reason::DevelopmentBuild))
    }
}
