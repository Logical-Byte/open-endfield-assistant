use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// 检查和下载更新时使用的服务。
///
/// MirrorChyan/GitHub 的产品选项改编自 PR #6；具体请求由 Rust 更新模块执行。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateSource {
    /// 优先使用 MirrorChyan；CDK 为空时工作流回退到 GitHub。
    #[default]
    Mirrorchyan,
    /// 直接使用项目的 GitHub Release。
    Github,
}

/// `reqwest` 客户端如何选择代理。
///
/// 这些用户设置改编自 PR #6 的下载配置界面。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateProxyMode {
    /// 禁用系统和环境提供的代理。
    None,
    /// 使用 `reqwest` 默认的系统代理发现行为。
    #[default]
    System,
    /// 仅使用 `update_proxy_url` 指定的代理。
    Custom,
}

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
    /// 启动时检查更新。
    #[serde(default = "default_check_updates")]
    pub check_updates: bool,
    #[serde(default)]
    /// 用户选择的更新元数据和完整包来源。
    pub update_source: UpdateSource,
    #[serde(default)]
    /// MirrorChyan 的访问凭据；为空时不向该服务发请求。
    pub mirrorchyan_cdk: String,
    #[serde(default)]
    /// 更新 HTTP 客户端使用的代理策略。
    pub update_proxy_mode: UpdateProxyMode,
    #[serde(default)]
    /// 自定义代理模式下传给 `reqwest` 的完整代理 URL。
    pub update_proxy_url: String,
}

/// `sound_volume` 字段默认值。
const fn default_sound_volume() -> f32 {
    0.5
}

/// 是否在应用启动后仅检查更新元数据的默认值。
const fn default_check_updates() -> bool {
    true
}

impl Default for OeaConfig {
    fn default() -> Self {
        Self {
            version: (0, 0),
            minimize_to_tray: false,
            sound_volume: default_sound_volume(),
            check_updates: default_check_updates(),
            update_source: UpdateSource::default(),
            mirrorchyan_cdk: String::new(),
            update_proxy_mode: UpdateProxyMode::default(),
            update_proxy_url: String::new(),
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
