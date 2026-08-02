//! 应用资源路径模块。
//!
//! 集中解析 `resources/`、`models/`、`logs/` 等资源目录的绝对路径，
//! 避免代码依赖运行时的工作目录（cwd）。
//!
//! 定位策略：
//! - 开发期：以 `CARGO_MANIFEST_DIR`（编译时指向 `src-tauri/`）的上一级作为项目根，
//!   再拼接各资源目录；
//! - 打包期：接入 Tauri 后改用 `app.path().resource_dir()` 定位打包资源。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 应用各资源目录的绝对路径。
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// 项目根目录
    pub root: PathBuf,
    /// 共享资源目录（前后端共用，submodule）
    pub resources_dir: PathBuf,
    /// OCR 模型目录
    pub models_dir: PathBuf,
    /// 运行日志目录
    pub logs_dir: PathBuf,
}

impl AppPaths {
    /// 基于开发期项目结构解析路径。
    ///
    /// `CARGO_MANIFEST_DIR` 在编译时指向 `src-tauri/`，其上一级即为项目根。
    pub fn new() -> Result<Self> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .context("无法定位项目根目录（src-tauri 的上一级）")?;

        Ok(Self {
            root: root.to_path_buf(),
            resources_dir: root.join("resources"),
            models_dir: root.join("models"),
            logs_dir: root.join("logs"),
        })
    }

    /// 模板图片根目录（`resources/templates`）
    pub fn templates_dir(&self) -> PathBuf {
        self.resources_dir.join("templates")
    }
}
