//! 音效播放接口。

use std::path::Path;

#[cfg(target_os = "windows")]
use super::windows;

/// 播放 wav 音效文件（异步，立即返回；`volume` 取值 0.0–1.0）。
/// macOS 开发外壳不执行任何操作。
pub fn play_wav(path: &Path, volume: f32) {
    #[cfg(target_os = "windows")]
    {
        windows::sound::play_wav(path, volume);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (path, volume);
    }
}
