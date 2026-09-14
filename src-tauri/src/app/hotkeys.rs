//! 应用热键定义与动作分发。

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tauri::{AppHandle, Manager};
use tracing::debug;

use crate::{controller::Controller, platform};

const RECEIVE_TIMEOUT: Duration = Duration::from_millis(100);

/// 引号 `'`：切换扫描档案库任务。
const TOGGLE_MAIN_TASK_HOTKEY: platform::hotkey::KeyEvent = platform::hotkey::KeyEvent {
    vk: platform::hotkey::OEM_7_KEY,
    down: true,
    modifiers: 0,
};

/// Alt+Delete：退出应用。
const EXIT_HOTKEY: platform::hotkey::KeyEvent = platform::hotkey::KeyEvent {
    vk: platform::hotkey::DELETE_KEY,
    down: true,
    modifiers: platform::hotkey::ALT_MODIFIER,
};

/// 启动热键动作分发线程。
///
/// `stop` 被设置为 `true` 后，线程会在观察到该值后退出。
pub(super) fn spawn_dispatcher(
    rx: Receiver<platform::hotkey::KeyEvent>,
    stop: Arc<AtomicBool>,
    foreground: platform::window::ForegroundGuard,
    app_handle: AppHandle,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("oea-hotkey".to_string())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let key_event = match rx.recv_timeout(RECEIVE_TIMEOUT) {
                    Ok(key_event) => key_event,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };

                if key_event == EXIT_HOTKEY {
                    let controller = app_handle.state::<Controller>();
                    controller.quit(&app_handle);
                } else if key_event == TOGGLE_MAIN_TASK_HOTKEY {
                    if foreground.is_foreground_eligible() {
                        let controller = app_handle.state::<Controller>();
                        controller.toggle_scan(&app_handle);
                    } else {
                        debug!("前台窗口不是终末地或者 OEA，忽略热键");
                    }
                }
            }
        })
        .expect("启动热键动作分发线程失败")
}
