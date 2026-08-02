//! OEA Assistant - 明日方舟终末地 自动化助手（Tauri 后端）。

// ============ 业务模块（原 dak 逻辑） ============
pub mod app;
pub mod app_paths;
pub mod geometry;
pub mod hotkey;
mod include;
pub mod input;
pub mod logger;
pub mod ocr;
pub mod resolution;
pub mod scene;
pub mod screencap;
pub mod session;
pub mod task;
pub mod tasks;
pub mod template_matching;
pub mod utils;
pub mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
