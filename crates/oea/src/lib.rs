//! OEA Assistant：共享 Rust 业务实现与原生 GPUI 桌面入口。

#[cfg(feature = "legacy-tauri")]
mod app;
#[cfg(feature = "desktop")]
mod desktop;

pub mod app_paths;
pub(crate) mod app_variant;
pub mod automation;
pub mod controller;
pub mod data;
pub mod locale;
pub mod navigation;
pub mod platform;
pub mod settings;
#[cfg(any(feature = "desktop", feature = "legacy-tauri"))]
mod startup;
pub(crate) mod storage;
pub mod update;
pub mod utils;

#[cfg(all(feature = "legacy-tauri", not(feature = "desktop")))]
pub use app::run;
#[cfg(feature = "legacy-tauri")]
pub use app::run as run_legacy;
#[cfg(feature = "desktop")]
pub use desktop::run;
