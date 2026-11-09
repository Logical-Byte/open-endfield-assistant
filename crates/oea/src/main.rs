// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    oea_lib::update::install::run_helper_if_requested();
    oea_lib::run();
    std::process::ExitCode::SUCCESS
}
