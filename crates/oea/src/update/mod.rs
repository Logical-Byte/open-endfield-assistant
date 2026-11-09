//! 自动更新：下载、进度、取消与安装状态。

use crate::app_variant::{self, AppVariant};

pub mod commands;
pub mod install;

pub mod error;
mod options;
mod status;

pub use error::Error;
pub use oea_update::Updater;

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
