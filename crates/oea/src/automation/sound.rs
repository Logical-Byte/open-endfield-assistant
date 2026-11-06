//! 自动化任务共用的提示音。

use std::{fs::File, io::BufReader, path::Path, sync::OnceLock};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, source::Source};
use tracing::{debug, warn};

use crate::app_paths::AppPaths;

/// 全局默认输出流（首次播放时懒初始化，保活到进程退出）。
static OUTPUT_SINK: OnceLock<Option<MixerDeviceSink>> = OnceLock::new();

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
    play_wav(&path, volume);
}

fn play_wav(path: &Path, volume: f32) {
    let sink = OUTPUT_SINK
        .get_or_init(|| DeviceSinkBuilder::open_default_sink().ok())
        .as_ref();
    let Some(sink) = sink else {
        warn!("打开默认音频输出设备失败，跳过音效: {}", path_for_log(path));
        return;
    };
    let Ok(file) = File::open(path) else {
        warn!("音效文件不存在: {}", path_for_log(path));
        return;
    };
    match Decoder::try_from(BufReader::new(file)) {
        Ok(source) => {
            sink.mixer().add(source.amplify(volume.clamp(0.0, 1.0)));
            debug!("播放音效: {}", path_for_log(path));
        }
        Err(error) => warn!("解码音效失败 ({}): {error}", path_for_log(path)),
    }
}

fn path_for_log(path: &Path) -> String {
    let path = path.display().to_string();
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Cue {
    Enable,
    Disable,
}
