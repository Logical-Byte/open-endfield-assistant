use crate::UiLocale;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 当前设置文件主要版本号
pub const CURRENT_MAJOR_VERSION: u32 = 0;
/// 当前设置文件次要版本号
pub const CURRENT_MINOR_VERSION: u32 = 2;

/// 更新源。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "settings/")]
pub enum UpdateSource {
    Mirrorchyan,
    #[default]
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
    /// 应用界面与原生提示使用的语言。
    pub ui_locale: UiLocale,
    /// 设置文件主要版本号，产生不兼容变更（改变字段结构或者删除字段）时，增加 `majorVersion` 的值
    pub major_version: u32,
    /// 设置文件次要版本号，设置格式或规则发生可迁移的兼容变更时，增加 `minorVersion` 的值
    pub minor_version: u32,
    /// 关闭窗口时最小化到托盘。
    pub minimize_to_tray: bool,
    /// 扫描音效音量（`0.0` ~ `1.0`）。
    pub sound_volume: f32,
    /// 用户选择的更新源。
    pub update_source: UpdateSource,
    /// Mirror酱 CDK 密文
    pub mirrorchyan_cdk_encrypted: String,
    /// 更新代理模式。
    pub update_proxy_mode: UpdateProxyMode,
    /// 更新代理 URL
    pub update_proxy_url: String,
    /// 是否自动下载更新。
    pub auto_download_updates: bool,
    /// 是否自动安装已下载的更新。
    pub auto_install_updates: bool,
    /// 档案扫描启动提示已确认的版本号。
    ///
    /// 提示内容版本与设置格式版本相互独立。需要用户重新确认提示时，
    /// 递增前端 `CURRENT_SCAN_TIPS_VERSION`，不增加设置格式版本。
    pub scan_tips_dismissed_version: u32,
}

impl Default for OeaSettings {
    fn default() -> Self {
        Self {
            ui_locale: UiLocale::ZhCn,
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
