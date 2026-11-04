//! 自动化任务共用的提示音。

use tracing::warn;

use crate::{app_paths::AppPaths, platform};

/// 播放自动化任务提示音（音量取本次设置快照）。
pub(super) fn play(volume: f32, sound: Cue) {
    let app_paths = match AppPaths::new() {
        Ok(app_paths) => app_paths,
        Err(error) => {
            warn!("无法解析自动化任务提示音资源: {error}");
            return;
        }
    };
    let relative_path = match sound {
        Cue::Enable => "sounds/enable.wav",
        Cue::Disable => "sounds/disable.wav",
    };
    let path = match app_paths.resolve_resource_file(relative_path) {
        Ok(path) => path,
        Err(error) => {
            warn!("无法解析自动化任务提示音资源: {error:#}");
            return;
        }
    };
    platform::sound::play_wav(&path, volume);
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Cue {
    Enable,
    Disable,
}
