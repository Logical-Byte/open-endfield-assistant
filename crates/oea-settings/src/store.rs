use crate::{
    OeaSettings, SettingsError, SettingsErrorKind, UiLocale, migrations,
    persistence::CachedJsonFile, validation::validate,
};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// 仅发布成功加载、迁移或保存的设置快照。
pub struct SettingsStore {
    file: CachedJsonFile<OeaSettings>,
}
impl SettingsStore {
    pub fn open(path: PathBuf, default_ui_locale: UiLocale) -> Result<Self, SettingsError> {
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Self {
                    file: CachedJsonFile::new(
                        path,
                        OeaSettings {
                            ui_locale: default_ui_locale,
                            ..OeaSettings::default()
                        },
                    ),
                });
            }
            Err(error) => return Err(SettingsError::new(SettingsErrorKind::Read, error)),
        };
        let mut document = serde_json::from_slice(&bytes)
            .map_err(|error| SettingsError::new(SettingsErrorKind::InvalidFile, error))?;
        let migrated = migrations::migrate(&mut document, default_ui_locale)?;
        // 对当前格式同样允许省略语言字段，默认值由应用明确提供。
        if let Some(object) = document.as_object_mut() {
            object
                .entry("uiLocale")
                .or_insert_with(|| serde_json::json!(default_ui_locale));
        }
        let settings: OeaSettings = serde_json::from_value(document)
            .map_err(|error| SettingsError::new(SettingsErrorKind::InvalidFile, error))?;
        validate(&settings)?;
        let store = Self {
            file: CachedJsonFile::new(path, settings.clone()),
        };
        if migrated {
            store.save(settings)?;
        }
        Ok(store)
    }
    pub fn snapshot(&self) -> OeaSettings {
        self.file.snapshot().as_ref().clone()
    }
    pub fn save(&self, settings: OeaSettings) -> Result<(), SettingsError> {
        validate(&settings)?;
        self.file
            .replace(settings)
            .map_err(|error| SettingsError::new(SettingsErrorKind::Write, format!("{error:#}")))
    }
    pub fn path(&self) -> &Path {
        self.file.path()
    }
}
/// 启动早期只读语言字段，不执行迁移，也不修改文件。
pub fn read_ui_locale(path: &Path) -> Result<Option<UiLocale>, SettingsError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(SettingsError::new(SettingsErrorKind::Read, error)),
    };
    let document: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| SettingsError::new(SettingsErrorKind::InvalidFile, error))?;
    document
        .get("uiLocale")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| SettingsError::new(SettingsErrorKind::InvalidFile, error))
}
