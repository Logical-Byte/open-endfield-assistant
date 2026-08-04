//! 应用资源路径模块。
//!
//! 集中解析 `resources/`、`models/`、`logs/` 等资源目录的绝对路径，
//! 避免代码依赖运行时的工作目录（cwd）。
//!
//! 定位策略（根目录下应有 `models`、`resources`、`logs` 三个文件夹）：
//! - 开发期（debug 构建）：以 `CARGO_MANIFEST_DIR`（编译时指向 `src-tauri/`）的
//!   上一级作为项目根，再拼接各资源目录；
//! - 打包期（release 构建）：以 exe 所在目录作为根目录（便携式分发，资源与 exe 同目录）。
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
    root: PathBuf,
}

impl AppPaths {
    /// 解析应用根目录。
    ///
    /// 根目录在运行时确定：
    /// - debug 构建（`tauri dev` / `cargo run`）：`CARGO_MANIFEST_DIR` 的上一级 = 项目根；
    /// - release 构建（`tauri build` / `cargo build --release`）：exe 所在目录。
    pub fn new() -> Result<Self> {
        Ok(Self {
            root: get_root_dir()?,
        })
    }

    /// 应用根目录。
    pub fn root_dir(&self) -> &Path {
        &self.root
    }

    /// 共享资源目录（前后端共用，submodule）。
    pub fn resources_dir(&self) -> PathBuf {
        self.root.join("resources")
    }

    /// OCR 模型目录。
    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    /// 运行日志目录。
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// WebView2 用户数据目录（绿色便携：不写入 `%LOCALAPPDATA%`）。
    pub fn webview_data_dir(&self) -> PathBuf {
        self.root.join("webview-data")
    }

    /// 模板图片根目录（`resources/templates`）。
    pub fn templates_dir(&self) -> PathBuf {
        self.resources_dir().join("templates")
    }
}
