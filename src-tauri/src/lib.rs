//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

mod app;

pub mod app_paths;
pub mod automation;
pub mod controller;
pub mod data;
pub mod logger;
pub mod navigation;
pub mod platform;
pub mod settings;
pub(crate) mod storage;
pub mod update;
pub mod utils;
pub(crate) mod vision;

pub use app::run;
