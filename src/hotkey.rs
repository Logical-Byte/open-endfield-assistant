use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use scopeguard::defer;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_DELETE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

/// 全局快捷键监听器。
/// 在独立线程中注册并监听热键，按下时设置停止标志。
pub struct HotkeyListener {
    stop_flag: Arc<AtomicBool>,
}

impl HotkeyListener {
    /// 创建并启动热键监听。
    /// `modifiers` 如 `MOD_ALT`，`vk` 如 `VK_DELETE`。
    pub fn new(modifiers: HOT_KEY_MODIFIERS, vk: u32) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let flag = stop_flag.clone();

        thread::spawn(move || {
            let hotkey_id = 1i32;
            unsafe {
                if RegisterHotKey(None, hotkey_id, modifiers, vk).is_err() {
                    eprintln!("热键注册失败（可能已被占用）");
                    return;
                }
            }
            defer! {
                unsafe { let _ = UnregisterHotKey(None, hotkey_id); }
            }

            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 == 0 || ret.0 == -1 {
                    break;
                }
                if msg.message == WM_HOTKEY {
                    flag.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });

        Self { stop_flag }
    }

    /// 停止快捷键（Alt + Delete）
    pub fn alt_delete() -> Self {
        Self::new(MOD_ALT | MOD_NOREPEAT, VK_DELETE.0 as u32)
    }

    /// 检查是否触发了停止
    pub fn stop_requested(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }
}
