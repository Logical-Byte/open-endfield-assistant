// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), not(feature = "cli")),
    windows_subsystem = "windows"
)]

fn main() -> std::process::ExitCode {
    #[cfg(feature = "cli")]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("dev")) {
        // `dev` owns its argv and exits before any Tauri initialization.
        return oea_lib::run_dev_cli(std::env::args_os().skip(1));
    }
    if let Some((root, executable_name, locale)) =
        oea_lib::update::install::helper_request_from_args(std::env::args_os())
    {
        if let Err(error) =
            oea_lib::update::install::run_helper_request_with_logging(root, executable_name, locale)
        {
            eprintln!("更新 helper 失败: {error}");
            std::process::exit(1);
        }
        return std::process::ExitCode::SUCCESS;
    }
    oea_lib::run();
    std::process::ExitCode::SUCCESS
}
