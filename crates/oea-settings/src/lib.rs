//! OEA 设置的格式、迁移、校验与原子持久化。
mod error;
mod locale;
mod migrations;
mod model;
mod persistence;
mod store;
mod validation;

pub use error::{SettingsError, SettingsErrorKind};
pub use locale::UiLocale;
pub use model::{
    CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION, OeaSettings, UpdateProxyMode, UpdateSource,
};
pub use store::{SettingsStore, read_ui_locale};
