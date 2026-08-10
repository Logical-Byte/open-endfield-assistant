// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 启动时自动请求管理员权限（仅 release；用户取消则继续以普通权限运行）
    #[cfg(target_os = "windows")]
    oea_lib::admin::elevate_at_startup();

    oea_lib::run();
}
