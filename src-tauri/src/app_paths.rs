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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 应用各资源目录的绝对路径。
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// 应用根目录
    pub root: PathBuf,
    /// 共享资源目录（前后端共用，submodule）
    pub resources_dir: PathBuf,
    /// OCR 模型目录
    pub models_dir: PathBuf,
    /// 运行日志目录
    pub logs_dir: PathBuf,
}

impl AppPaths {
    /// 解析应用根目录并拼接各资源目录。
    ///
    /// 根目录在运行时确定：
    /// - debug 构建（`tauri dev` / `cargo run`）：`CARGO_MANIFEST_DIR` 的上一级 = 项目根；
    /// - release 构建（`tauri build` / `cargo build --release`）：exe 所在目录。
    pub fn new() -> Result<Self> {
        let root = resolve_root()?;
        Ok(Self::from_root(root))
    }

    /// 基于指定根目录构造路径（根目录下应包含 `models`/`resources`/`logs`）。
    fn from_root(root: PathBuf) -> Self {
        Self {
            resources_dir: root.join("resources"),
            models_dir: root.join("models"),
            logs_dir: root.join("logs"),
            root,
        }
    }

    /// 模板图片根目录（`resources/templates`）
    pub fn templates_dir(&self) -> PathBuf {
        self.resources_dir.join("templates")
    }
}

/// 在运行时确定应用根目录。
///
/// - debug 构建：`CARGO_MANIFEST_DIR`（编译期指向 `src-tauri/`）的上一级即项目根；
/// - release 构建：`current_exe()` 所在目录即分发根目录。
fn resolve_root() -> Result<PathBuf> {
    if cfg!(debug_assertions) {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .map(Path::to_path_buf)
            .context("无法定位项目根目录（src-tauri 的上一级）")
    } else {
        let exe_path = std::env::current_exe().context("无法获取当前 exe 路径")?;
        exe_path
            .parent()
            .map(Path::to_path_buf)
            .context("无法定位 exe 所在目录")
    }
}
