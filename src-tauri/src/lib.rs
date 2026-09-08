//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

mod app;

pub mod app_paths;
pub mod automation;
pub mod config;
pub mod controller;
pub mod data;
pub mod logger;
pub mod ocr;
pub mod platform;
pub(crate) mod scan_runtime;
pub mod scene;
pub mod session;
pub mod task;
pub mod template_matching;
pub mod update;
pub mod utils;

pub use app::run;
