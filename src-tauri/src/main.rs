// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 启动时自动请求管理员权限（仅 release；用户取消则继续以普通权限运行）
    #[cfg(target_os = "windows")]
    oea_lib::windows_ops::admin::elevate_at_startup();

    // 尽早安装全局 panic hook：任何 panic（含 Tauri setup 失败导致的 panic）都会
    // 独立写入 logs/crash-*.log，保证 release（无控制台）下也有可回溯记录。
    oea_lib::crash::install_panic_hook();

    oea_lib::run();
}
