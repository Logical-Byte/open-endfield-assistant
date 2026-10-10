use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::warn;
use ts_rs::TS;

use crate::storage::CachedJsonFile;

/// 当前设置文件主要版本号
pub const CURRENT_MAJOR_VERSION: u32 = 0;
/// 当前设置文件次要版本号
pub const CURRENT_MINOR_VERSION: u32 = 1;

/// 更新源。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "settings/")]
pub enum UpdateSource {
    #[default]
    Mirrorchyan,
    Oem,
    Github,
}

/// 代理模式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "settings/")]
pub enum UpdateProxyMode {
    None,
    #[default]
    System,
    Custom,
}

/// OEA 用户设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "settings/")]
pub struct OeaSettings {
    /// 应用语言。缺失字段采用当前用户系统界面语言。
    pub ui_locale: crate::locale::UiLocale,
    /// 设置文件主要版本号，产生不兼容变更（改变字段结构或者删除字段）时，增加 `majorVersion` 的值
    pub major_version: u32,
    /// 设置文件次要版本号，产生兼容变更（添加新字段但不改变原有字段的结构）时，增加 `minorVersion` 的值
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
    /// 与前端 `ScanGuide.vue` 的常量 `CURRENT_SCAN_TIPS_VERSION` 配合：
    /// 前端在 `scan_tips_dismissed_version < 当前提示版本` 时展示启动提示；
    /// 用户勾选「下次更新前不再提示」并确认后，前端把本字段更新为当前提示版本并持久化。
    ///
    /// 更新提示文案时，想让所有用户（含已确认过的）重新看一次，只需递增前端
    /// `CURRENT_SCAN_TIPS_VERSION`，本字段无需改动；只改文案不递增版本号则老用户不重看。
    /// 本字段的「值」不属于 settings 结构变更，不会因此再次 bump `minorVersion`。
    pub scan_tips_dismissed_version: u32,
}

impl Default for OeaSettings {
    fn default() -> Self {
        Self {
            ui_locale: crate::locale::UiLocale::default(),
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

/// OEA 设置的内存缓存与持久化入口。
///
/// 设置默认值和错误回退等领域语义由本模块负责。JSON 格式、并发缓存和文件提交交给
/// 通用的 [`CachedJsonFile`]。
pub struct SettingsStore {
    file: CachedJsonFile<OeaSettings>,
}

impl SettingsStore {
    /// 从设置文件创建存储。文件不存在或内容无效时以默认设置初始化内存缓存。
    ///
    /// 回退默认值不会立即覆盖磁盘上的无效内容。只有显式调用 [`Self::save`] 才会写盘。
    pub fn at(path: PathBuf) -> Self {
        let file = match CachedJsonFile::at(path.clone()) {
            Ok(file) => file,
            Err(error) if is_not_found(&error) => CachedJsonFile::new(path, OeaSettings::default()),
            Err(error) => {
                warn!(error = %error, path = %path.display(), "加载设置文件失败，使用默认设置");
                CachedJsonFile::new(path, OeaSettings::default())
            }
        };
        Self { file }
    }

    /// 返回当前设置的独立快照。
    pub fn snapshot(&self) -> OeaSettings {
        self.file.snapshot().as_ref().clone()
    }

    /// 保存完整设置。文件提交成功后才会发布新的内存快照。
    pub fn save(&self, settings: OeaSettings) -> anyhow::Result<()> {
        self.file.replace(settings)
    }

    pub fn path(&self) -> &Path {
        self.file.path()
    }
}

/// 启动早期只读取语言字段，不初始化缓存或覆盖设置文件。
/// 其他设置字段损坏不影响有效语言值。缺失、无效或读取失败使用系统语言。
pub(crate) fn read_ui_locale(path: &Path) -> crate::locale::UiLocale {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value.get("uiLocale").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(crate::locale::UiLocale::system_default)
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
    fn test_load_nonexistent_settings() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nonexistent_settings.json");
        let store = SettingsStore::at(path);
        assert_eq!(store.snapshot(), OeaSettings::default());
    }

    #[test]
    fn test_save_and_load_settings() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("test_settings.json");
        let original_settings = OeaSettings {
            ui_locale: crate::locale::UiLocale::EnUs,
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
        let store = SettingsStore::at(path.clone());
        store.save(original_settings.clone()).unwrap();

        assert_eq!(store.snapshot(), original_settings);
        assert_eq!(SettingsStore::at(path).snapshot(), original_settings);
    }

    #[test]
    fn test_load_invalid_settings() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("invalid_settings.json");
        fs::write(&path, "invalid json").unwrap();
        let store = SettingsStore::at(path);
        assert_eq!(store.snapshot(), OeaSettings::default());
    }

    #[test]
    fn test_deserialize_settings() {
        let json = r#"
        {
            "majorVersion": 1,
            "minorVersion": 2,
            "minimizeToTray": true,
            "soundVolume": 0.7
        }
        "#;
        let settings: OeaSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.major_version, 1);
        assert_eq!(settings.minor_version, 2);
        assert!(settings.minimize_to_tray);
        assert_eq!(settings.sound_volume, 0.7);
        assert_eq!(settings.update_source, UpdateSource::default());
        assert_eq!(settings.mirrorchyan_cdk_encrypted, "".to_string());
        assert_eq!(settings.update_proxy_mode, UpdateProxyMode::default());
        assert_eq!(settings.update_proxy_url, "".to_string());
        assert!(settings.auto_download_updates);
        assert!(settings.auto_install_updates);
        assert_eq!(settings.scan_tips_dismissed_version, 0);
    }
}
