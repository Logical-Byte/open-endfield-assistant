//! 应用侧设置初始化与系统默认语言，持久化规则由 `oea-settings` 负责。
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use crate::{locale::UiLocale, platform};
pub use oea_settings::{
    OeaSettings, SettingsError, SettingsErrorKind, SettingsStore, UpdateProxyMode, UpdateSource,
};

/// 只缓存成功创建的存储；失败后允许重新读取文件并执行迁移。
pub struct SettingsAccess {
    path: PathBuf,
    store: OnceLock<SettingsStore>,
    initialization: Mutex<()>,
}
impl SettingsAccess {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            store: OnceLock::new(),
            initialization: Mutex::new(()),
        }
    }
    pub fn initialize(&self) -> Result<&SettingsStore, SettingsError> {
        if let Some(store) = self.store.get() {
            return Ok(store);
        }
        let _guard = self
            .initialization
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(store) = self.store.get() {
            return Ok(store);
        }
        let store = SettingsStore::open(self.path.clone(), system_ui_locale())?;
        let _ = self.store.set(store);
        Ok(self.store.get().expect("存储已在初始化锁内发布"))
    }
    pub fn get(&self) -> Result<&SettingsStore, SettingsError> {
        self.store.get().ok_or_else(|| {
            SettingsError::new(
                SettingsErrorKind::Unavailable,
                "设置尚不可用，请在设置页重试加载",
            )
        })
    }
}

pub(crate) fn system_ui_locale() -> UiLocale {
    platform::locale::user_interface_language()
        .as_deref()
        .map(UiLocale::from_language_tag)
        .unwrap_or(UiLocale::ZhCn)
}

/// 原生启动提示的语言读取独立于完整设置初始化，不迁移、不写盘。
pub(crate) fn read_ui_locale(path: &Path) -> UiLocale {
    oea_settings::read_ui_locale(path)
        .ok()
        .flatten()
        .unwrap_or_else(system_ui_locale)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialization_can_retry_after_file_repair_and_preserves_successful_store() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        std::fs::write(&path, "broken").unwrap();
        let access = SettingsAccess::new(path.clone());
        assert!(access.initialize().is_err());
        assert!(access.get().is_err());
        std::fs::write(
            &path,
            r#"{"majorVersion":0,"minorVersion":1,"soundVolume":0.8}"#,
        )
        .unwrap();
        assert_eq!(access.initialize().unwrap().snapshot().sound_volume, 0.8);
        std::fs::write(&path, "external change").unwrap();
        assert_eq!(access.initialize().unwrap().snapshot().sound_volume, 0.8);
    }
}
