use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// 应用配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OeaConfig {
    /// 配置文件版本号（用于升级时迁移配置）
    pub version: (u32, u32),
    /// 关闭时最小化到托盘而不是退出应用
    pub minimize_to_tray: bool,
    /// 扫描音效音量（0.0–1.0）
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,
}

/// `sound_volume` 字段默认值。
const fn default_sound_volume() -> f32 {
    0.5
}

impl Default for OeaConfig {
    fn default() -> Self {
        Self {
            version: (0, 0),
            minimize_to_tray: false,
            sound_volume: default_sound_volume(),
        }
    }
}

/// 从配置文件加载；文件不存在或解析失败时回退默认配置。
pub fn load_oea_config(path: &Path) -> OeaConfig {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            warn!("解析配置文件失败，使用默认配置: {e}");
            OeaConfig::default()
        }),
        Err(e) => {
            warn!("读取配置文件失败，使用默认配置: {e}");
            OeaConfig::default()
        }
    }
}

/// 保存到配置文件（自动创建父目录）。
pub fn save_oea_config(config: &OeaConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建配置文件父目录失败")?;
    }
    let text = serde_json::to_string_pretty(config).context("序列化配置失败")?;
    fs::write(path, text).context("写入配置文件失败")?;
    Ok(())
}
