//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

mod app;

#[cfg(feature = "cli")]
mod dev_cli;
#[cfg(feature = "cli")]
pub use dev_cli::run_dev_cli;

pub mod app_paths;
pub mod automation;
pub mod controller;
pub mod data;
pub mod locale;
pub mod logger;
pub mod navigation;
pub mod platform;
pub mod settings;
pub(crate) mod storage;
pub mod update;
pub mod utils;
pub(crate) mod vision;

pub use app::run;
