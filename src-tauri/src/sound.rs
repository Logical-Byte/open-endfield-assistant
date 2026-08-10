//! 音效播放（rodio 实现，支持音量调节）。
//!
//! 全局持有一个默认输出流（保活到应用退出；输出流被 drop 播放即停），
//! 每次播放将解码后的音源直接混入输出流：`Mixer::add` 即播即忘、播完自动释放。
//! 音量通过 `amplify` 逐样本增益实现，由 [`crate::config::OeaConfig::sound_volume`] 配置。

use std::{fs::File, io::BufReader, path::Path, sync::OnceLock};

use rodio::{Decoder, OutputStream, OutputStreamBuilder, source::Source};
use tracing::{debug, warn};

/// 全局默认输出流（首次播放时懒初始化，保活到进程退出）。
static OUTPUT_STREAM: OnceLock<Option<OutputStream>> = OnceLock::new();

/// 播放 wav 音效文件（异步，立即返回；`volume` 取值 0.0–1.0）。
pub fn play_wav(path: &Path, volume: f32) {
    let stream = OUTPUT_STREAM
        .get_or_init(|| OutputStreamBuilder::open_default_stream().ok())
        .as_ref();
    let Some(stream) = stream else {
        warn!("打开默认音频输出设备失败，跳过音效: {}", path.display());
        return;
    };
    let Ok(file) = File::open(path) else {
        warn!("音效文件不存在: {}", path.display());
        return;
    };
    match Decoder::try_from(BufReader::new(file)) {
        Ok(source) => {
            stream.mixer().add(source.amplify(volume.clamp(0.0, 1.0)));
            debug!("播放音效: {}", path.display());
        }
        Err(e) => warn!("解码音效失败 ({}): {e}", path.display()),
    }
}
