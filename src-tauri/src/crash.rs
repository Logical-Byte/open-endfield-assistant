//! 致命错误处理：panic hook 与启动失败兜底。
//!
//! 背景：release 构建使用 `windows_subsystem = "windows"` 隐藏控制台，且 Tauri v2 的
//! setup 失败会直接 `panic!("Failed to setup app: ...")`（见 tauri `app.rs` 的
//! `RuntimeRunEvent::Ready` 分支）。若不做任何处理，用户只会看到程序"闪退"，
//! 没有任何可回溯的信息。
//!
//! 本模块提供两层兜底，保证任何致命错误都"留痕 + 可见"：
//! - [`install_panic_hook`]：全局 panic hook，把 panic 消息与 backtrace 独立写入
//!   `logs/crash-<时间>-<pid>.log`（不依赖 tracing，panic 发生时日志系统可能尚未初始化），
//!   并同步输出到 tracing 管道（若已初始化，可进入当日日志与前端）。
//! - [`report_fatal`]：确定性致命错误（如 setup 失败）的统一出口——全链错误写入
//!   日志与 crash 文件，弹原生对话框告知用户，最后退出进程。
//!
//! crash 文件与当日日志相互独立：当日日志依赖 tracing 初始化，crash 文件只用
//! `std::fs` 直接写，是 panic / 早期初始化失败时的最后退路。

use std::backtrace::Backtrace;
use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use chrono::Local;

use crate::app_paths::AppPaths;
use crate::window::dialog::{self, DialogIcon};

static PANIC_HOOK_INSTALLED: Once = Once::new();

/// 安装全局 panic hook（应在应用入口尽早调用，且只安装一次）。
///
/// 行为：
/// - 先调用默认 hook（保证 `tauri dev` 等带控制台场景下的行为不变）；
/// - 把 panic 消息 + backtrace 独立写入 `logs/crash-<时间戳>-<pid>.log`（时间戳含毫秒
///   精度、文件名带进程 ID，避免同一秒内多次崩溃互相覆盖；日志目录不可用时回退到
///   系统临时目录，保证一定有记录）；
/// - 同步输出到 tracing 管道（若已初始化）。
///
/// hook 内全部用不可 panic 的写法，避免 hook 自身再触发 abort。
pub fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            default_hook(info);

            let backtrace = Backtrace::force_capture();
            let body = format!("panic: {info}\n\nbacktrace:\n{backtrace}");
            let _ = write_crash_log("PANIC", &body);
            tracing::error!("{body}");
        }));
    });
}

/// 确定性致命错误（如 setup 失败）的统一出口。
///
/// 1. 全链错误（`{e:#}`）写入 tracing 管道（会进当日日志与前端）；
/// 2. 独立写入 crash 文件（即使 tracing 未初始化也有记录）；
/// 3. 弹原生错误对话框，展示错误详情与日志文件位置（release 无控制台时用户也能看到）；
/// 4. 退出进程（不返回）。
pub fn report_fatal(error: &anyhow::Error, app_handle: &tauri::AppHandle) -> ! {
    tracing::error!("启动初始化失败:\n{error:#}");

    let crash_file = write_crash_log("FATAL SETUP ERROR", &format!("{error:#}"));
    let crash_file_hint = match &crash_file {
        Some(path) => path.display().to_string(),
        None => "（写入失败）".to_string(),
    };

    let _ = dialog::show_message(
        "OEA 启动失败",
        &format!(
            "程序初始化失败，无法启动。\n\n错误详情：\n{error:#}\n\n详细日志已保存至：\n{crash_file_hint}",
        ),
        DialogIcon::Error,
    );

    // 兜底退出：setup 失败后进程不应继续运行
    app_handle.exit(1);
    std::process::exit(1);
}

/// 把一段致命错误文本写入 `logs/crash-<时间>-<pid>.log`。
///
/// 时间含毫秒精度（`%Y%m%d-%H%M%S-%f`），文件名再带上进程 ID：即使同一秒内多次
/// panic（或并发测试）也不会互相覆盖。优先写日志目录（与当日日志同目录，用户容易
/// 找到）；失败（如目录不可写 / 无法定位）时回退到系统临时目录，保证尽量留下记录。
/// 返回实际写入的文件路径。
fn write_crash_log(title: &str, body: &str) -> Option<PathBuf> {
    let candidates = [
        AppPaths::new().map(|p| p.logs_dir()).ok(),
        Some(std::env::temp_dir()),
    ];
    let time_string = Local::now().format("%Y%m%d-%H%M%S-%f");
    let pid = std::process::id();

    for dir in candidates.into_iter().flatten() {
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("crash-{time_string}-{pid}.log"));
        let content = format!("===== {title} =====\n{body}\n");
        if fs::write(&path, content).is_ok() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 收集 logs 目录下所有 `crash-*.log` 文件名。
    fn crash_files() -> Vec<PathBuf> {
        let logs_dir = AppPaths::new().unwrap().logs_dir();
        fs::read_dir(logs_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string());
                matches!(name.as_deref(), Some(n) if n.starts_with("crash-") && n.ends_with(".log"))
            })
            .collect()
    }

    #[test]
    fn write_crash_log_creates_file() {
        let path = write_crash_log("TEST", "hello crash").expect("crash 文件应写入成功");
        let content = fs::read_to_string(&path).expect("读取 crash 文件失败");
        assert!(content.contains("TEST"));
        assert!(content.contains("hello crash"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn panic_hook_writes_crash_file() {
        install_panic_hook();
        // 触发一次 panic 并捕获：hook 应在崩溃前把消息写入 crash 文件
        let _ = std::panic::catch_unwind(|| panic!("测试 panic 消息 42"));

        let matched = crash_files().iter().any(|p| {
            fs::read_to_string(p)
                .map(|c| c.contains("测试 panic 消息 42"))
                .unwrap_or(false)
        });
        assert!(matched, "panic hook 应把 panic 消息写入 crash 文件");
    }
}
