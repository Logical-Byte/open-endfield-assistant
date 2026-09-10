// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some((root, executable_name)) =
        oea_lib::update::install::helper_request_from_args(std::env::args_os())
    {
        let workspace =
            oea_lib::update::install::UpdateWorkspace::with_executable_name(root, executable_name);
        if let Err(error) = oea_lib::update::install::run_helper(&workspace) {
            eprintln!("更新 helper 失败: {error}");
            std::process::exit(1);
        }
        return;
    }
    oea_lib::run();
}
