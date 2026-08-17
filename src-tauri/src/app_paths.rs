//! 应用资源路径模块。
//!
//! 集中解析版本化资源、模型、日志等目录的绝对路径，
//! 避免代码依赖运行时的工作目录（cwd）。
//!
//! 定位策略：
//! - 开发期（debug 构建）：以 `CARGO_MANIFEST_DIR`（编译时指向 `src-tauri/`）的
//!   上一级作为项目根，再拼接各资源目录；
//! - 打包期（release 构建）：以 exe 所在目录作为根目录，资源位于
//!   `assets/v<内置版本>/models` 与 `assets/v<内置版本>/resources`。
//!
//! 注意：不能用 `env!("CARGO_MANIFEST_DIR")` 直接当作运行时路径——它是编译期常量，
//! 会把开发机路径（如 `D:\BioHazard\...`）烧进二进制，换机器后必然失效。

use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// 在运行时确定应用根目录，按构建类型分发。
///
/// - debug 构建：`CARGO_MANIFEST_DIR`（编译期指向 `src-tauri/`）的上一级即项目根；
/// - release 构建：`current_exe()` 所在目录即分发根目录。
fn get_root_dir() -> Result<PathBuf> {
    if cfg!(debug_assertions) {
        get_root_dir_for_dev()
    } else {
        get_root_dir_for_release()
    }
}

/// 开发期（debug 构建）根目录：`CARGO_MANIFEST_DIR` 的上一级即项目根。
fn get_root_dir_for_dev() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .context("无法定位项目根目录（src-tauri 的上一级）")
        .map(Path::to_path_buf)
}

/// 打包期（release 构建）根目录：exe 所在目录即分发根目录。
fn get_root_dir_for_release() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("无法获取当前 exe 路径")?;
    exe_path
        .parent()
        .context("无法定位 exe 所在目录")
        .map(Path::to_path_buf)
}

/// 应用各资源目录的绝对路径。
///
/// 仅持有应用根目录，各资源目录由方法按需拼接。
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// 应用根目录
    root_dir: PathBuf,
    /// 正式构建使用的内置版本号；开发构建为 `None`。
    release_version: Option<String>,
}

impl AppPaths {
    /// 解析应用根目录。
    ///
    /// 根目录在运行时确定：
    /// - debug 构建（`tauri dev` / `cargo run`）：`CARGO_MANIFEST_DIR` 的上一级 = 项目根；
    /// - release 构建（`tauri build` / `cargo build --release`）：exe 所在目录。
    pub fn new() -> Result<Self> {
        Ok(Self::for_build(
            get_root_dir()?,
            env!("CARGO_PKG_VERSION"),
            cfg!(debug_assertions),
        ))
    }

    pub fn with_root_dir(root_dir: impl Into<PathBuf>) -> Self {
        Self::for_build(root_dir, env!("CARGO_PKG_VERSION"), cfg!(debug_assertions))
    }

    /// 以显式构建模式构造路径，用于验证开发与正式资源布局。
    pub fn for_build(
        root_dir: impl Into<PathBuf>,
        version: impl Into<String>,
        development: bool,
    ) -> Self {
        Self {
            root_dir: root_dir.into(),
            release_version: (!development).then(|| version.into()),
        }
    }

    /// 应用根目录。
    /// - debug 构建（`tauri dev` / `cargo run`）：`CARGO_MANIFEST_DIR` 的上一级 = 项目根；
    /// - release 构建（`tauri build` / `cargo build --release`）：exe 所在目录。
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn asset_root(&self) -> PathBuf {
        self.release_version.as_ref().map_or_else(
            || self.root_dir.clone(),
            |version| self.root_dir.join("assets").join(format!("v{version}")),
        )
    }

    /// 共享资源目录。开发构建位于根目录，正式构建位于内置版本目录。
    pub fn resources_dir(&self) -> PathBuf {
        self.asset_root().join("resources")
    }

    /// OCR 模型目录。开发构建位于根目录，正式构建位于内置版本目录。
    pub fn models_dir(&self) -> PathBuf {
        self.asset_root().join("models")
    }

    /// 运行日志目录（`<root_dir>/logs`）。
    pub fn logs_dir(&self) -> PathBuf {
        self.root_dir.join("logs")
    }

    /// 缓存目录（`<root_dir>/cache`，存放运行期缓存与临时文件，如 WebView2 安装引导程序）。
    pub fn cache_dir(&self) -> PathBuf {
        self.root_dir.join("cache")
    }

    /// WebView2 用户数据目录（`<root_dir>/cache/webview-data`，不写入 `%LOCALAPPDATA%`）。
    pub fn webview_data_dir(&self) -> PathBuf {
        self.cache_dir().join("webview-data")
    }

    /// 模板图片根目录。正式构建位于内置版本资源目录。
    pub fn templates_dir(&self) -> PathBuf {
        self.resources_dir().join("templates")
    }

    /// 配置文件目录（`<root_dir>/config`）。
    pub fn config_dir(&self) -> PathBuf {
        self.root_dir.join("config")
    }

    /// OEA 应用配置文件（`<root_dir>/config/oea_config.json`）。
    pub fn oea_config_file(&self) -> PathBuf {
        self.config_dir().join("oea_config.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_resources_stay_at_the_repository_root() {
        let paths = AppPaths::for_build(PathBuf::from("/repo"), "1.2.3", true);

        assert_eq!(paths.models_dir(), PathBuf::from("/repo/models"));
        assert_eq!(paths.resources_dir(), PathBuf::from("/repo/resources"));
    }

    #[test]
    fn release_resources_are_selected_by_embedded_version() {
        let paths = AppPaths::for_build(PathBuf::from("/portable"), "1.2.3", false);

        assert_eq!(
            paths.models_dir(),
            PathBuf::from("/portable/assets/v1.2.3/models")
        );
        assert_eq!(
            paths.resources_dir(),
            PathBuf::from("/portable/assets/v1.2.3/resources")
        );
    }
}
