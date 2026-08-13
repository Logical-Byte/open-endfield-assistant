use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// 当前配置文件主要版本号
pub const CURRENT_MAJOR_VERSION: u32 = 0;
/// 当前配置文件次要版本号
pub const CURRENT_MINOR_VERSION: u32 = 0;

/// 更新源。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    #[default]
    Mirrorchyan,
    Github,
}

/// 代理模式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateProxyMode {
    None,
    #[default]
    System,
    Custom,
}

/// 应用配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OeaConfig {
    /// 配置文件主要版本号，产生不兼容变更（改变字段结构或者删除字段）时，增加 `majorVersion` 的值
    pub major_version: u32,
    /// 配置文件次要版本号，产生兼容变更（添加新字段但不改变原有字段的结构）时，增加 `minorVersion` 的值
    pub minor_version: u32,
    /// 关闭时最小化到托盘而不是退出应用，默认 `false`
    pub minimize_to_tray: bool,
    /// 扫描音效音量（`0.0` ~ `1.0`，默认 `0.5`）
    pub sound_volume: f32,
    /// 更新源，默认 `mirrorchyan`
    pub update_source: UpdateSource,
    /// Mirror酱 CDK 密文
    pub mirrorchyan_cdk_encrypted: String,
    /// 更新代理模式，默认 `system`
    pub update_proxy_mode: UpdateProxyMode,
    /// 更新代理 URL
    pub update_proxy_url: String,
}

impl Default for OeaConfig {
    fn default() -> Self {
        Self {
            major_version: CURRENT_MAJOR_VERSION,
            minor_version: CURRENT_MINOR_VERSION,
            minimize_to_tray: false,
            sound_volume: 0.5,
            update_source: UpdateSource::default(),
            mirrorchyan_cdk_encrypted: "".to_string(),
            update_proxy_mode: UpdateProxyMode::default(),
            update_proxy_url: "".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_config() {
        let path = Path::new("nonexistent_config.json");
        let config = load_oea_config(path);
        assert_eq!(config, OeaConfig::default());
    }

    #[test]
    fn test_save_and_load_config() {
        let path = Path::new("test_config.json");
        let original_config = OeaConfig {
            major_version: 1,
            minor_version: 0,
            minimize_to_tray: true,
            sound_volume: 0.8,
            update_source: UpdateSource::Github,
            mirrorchyan_cdk_encrypted: "encrypted_cdk".to_string(),
            update_proxy_mode: UpdateProxyMode::default(),
            update_proxy_url: "http://localhost:8080".to_string(),
        };
        save_oea_config(&original_config, path).unwrap();
        let loaded_config = load_oea_config(path);
        assert_eq!(loaded_config, original_config);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_invalid_config() {
        let path = Path::new("invalid_config.json");
        fs::write(path, "invalid json").unwrap();
        let config = load_oea_config(path);
        assert_eq!(config, OeaConfig::default());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_deserialize_config() {
        let json = r#"
        {
            "majorVersion": 1,
            "minorVersion": 2,
            "minimizeToTray": true,
            "soundVolume": 0.7
        }
        "#;
        let config: OeaConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.major_version, 1);
        assert_eq!(config.minor_version, 2);
        assert_eq!(config.minimize_to_tray, true);
        assert_eq!(config.sound_volume, 0.7);
        assert_eq!(config.update_source, UpdateSource::default());
        assert_eq!(config.mirrorchyan_cdk_encrypted, "".to_string());
        assert_eq!(config.update_proxy_mode, UpdateProxyMode::default());
        assert_eq!(config.update_proxy_url, "".to_string());
    }
}
