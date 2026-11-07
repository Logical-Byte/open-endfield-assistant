//! Windows-heavy 实现的私有区域。

pub(super) mod admin;
pub(super) mod dialog;
pub(super) mod dpapi;
pub(super) mod file;
pub(super) mod hotkey;
pub(super) mod proxy;
pub(super) mod registry;
pub(super) mod sound;
pub(super) mod update;
#[cfg(feature = "legacy-tauri")]
pub(super) mod webview2;
pub(super) mod window;

pub(super) mod locale;
