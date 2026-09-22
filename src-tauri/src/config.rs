use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::storage::CachedJsonFile;

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
    Oem,
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
    /// 是否自动下载更新（默认 `true`）
    pub auto_download_updates: bool,
    /// 是否自动安装更新（默认 `true`；扫描任务运行中不会安装，等待扫描结束后自动安装）
    pub auto_install_updates: bool,
    /// 档案扫描启动提示已确认的版本号，默认 `0`。
    ///
    /// 前端 settings 持久化实现中的 `CURRENT_SCAN_TIPS_VERSION` 将本字段投影为逻辑
    /// `scanGuideEnabled`；`ScanGuide.vue` 只读取该逻辑设置。用户勾选「下次更新前不再提示」
    /// 并确认后，settings 单写者把本字段更新为当前提示版本并持久化。
    ///
    /// 更新提示文案时，想让所有用户（含已确认过的）重新看一次，只需递增前端
    /// `CURRENT_SCAN_TIPS_VERSION`，本字段无需改动；只改文案不递增版本号则老用户不重看。
    /// 本字段的「值」不属于 config 结构变更，不会因此再次 bump `minorVersion`。
    pub scan_tips_dismissed_version: u32,
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
            auto_download_updates: true,
            auto_install_updates: true,
            scan_tips_dismissed_version: 0,
        }
    }
}

/// OEA 配置的内存缓存与持久化入口。
///
/// 配置默认值和错误回退等领域语义由本模块负责；JSON 格式、并发缓存和文件提交交给
/// 通用的 [`CachedJsonFile`]。
pub struct ConfigStore {
    file: CachedJsonFile<OeaConfig>,
}

impl ConfigStore {
    /// 从配置文件创建存储；文件不存在或内容无效时以默认配置初始化内存缓存。
    ///
    /// 回退默认值不会立即覆盖磁盘上的无效内容。只有显式调用 [`Self::save`] 才会写盘。
    pub fn at(path: PathBuf) -> Self {
        let file = match CachedJsonFile::at(path.clone()) {
            Ok(file) => file,
            Err(error) if is_not_found(&error) => CachedJsonFile::new(path, OeaConfig::default()),
            Err(error) => {
                warn!(error = %error, path = %path.display(), "加载配置文件失败，使用默认配置");
                CachedJsonFile::new(path, OeaConfig::default())
            }
        };
        Self { file }
    }

    /// 返回当前配置的独立快照。
    pub fn snapshot(&self) -> OeaConfig {
        self.file.snapshot().as_ref().clone()
    }

    /// 保存完整配置；文件提交成功后才会发布新的内存快照。
    pub fn save(&self, config: OeaConfig) -> Result<()> {
        self.file.replace(config)
    }

    pub fn path(&self) -> &Path {
        self.file.path()
    }
}

fn is_not_found(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == ErrorKind::NotFound)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_load_nonexistent_config() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nonexistent_config.json");
        let store = ConfigStore::at(path);
        assert_eq!(store.snapshot(), OeaConfig::default());
    }

    #[test]
    fn test_save_and_load_config() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("test_config.json");
        let original_config = OeaConfig {
            major_version: 1,
            minor_version: 0,
            minimize_to_tray: true,
            sound_volume: 0.8,
            update_source: UpdateSource::Github,
            mirrorchyan_cdk_encrypted: "encrypted_cdk".to_string(),
            update_proxy_mode: UpdateProxyMode::default(),
            update_proxy_url: "http://localhost:8080".to_string(),
            auto_download_updates: false,
            auto_install_updates: false,
            scan_tips_dismissed_version: 1,
        };
        let store = ConfigStore::at(path.clone());
        store.save(original_config.clone()).unwrap();

        assert_eq!(store.snapshot(), original_config);
        assert_eq!(ConfigStore::at(path).snapshot(), original_config);
    }

    #[test]
    fn test_load_invalid_config() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("invalid_config.json");
        fs::write(&path, "invalid json").unwrap();
        let store = ConfigStore::at(path);
        assert_eq!(store.snapshot(), OeaConfig::default());
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
        assert!(config.minimize_to_tray);
        assert_eq!(config.sound_volume, 0.7);
        assert_eq!(config.update_source, UpdateSource::default());
        assert_eq!(config.mirrorchyan_cdk_encrypted, "".to_string());
        assert_eq!(config.update_proxy_mode, UpdateProxyMode::default());
        assert_eq!(config.update_proxy_url, "".to_string());
        assert!(config.auto_download_updates);
        assert!(config.auto_install_updates);
        assert_eq!(config.scan_tips_dismissed_version, 0);
    }
}
