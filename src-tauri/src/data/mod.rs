//! 静态数据文件统一管理。
//!
//! 集中加载 `resources/data/` 下的运行时数据文件（如 `prts.json`、
//! `archive_contract.json`），统一读取、解析、日志与错误处理。
//!
//! 应用启动时调用 [`AppData::load`] 一次，之后各模块只读共享（E1 严格策略：
//! 任一数据文件缺失或损坏即视为致命错误，启动失败并指明是哪个文件）。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use tracing::info;

use crate::app_paths::AppPaths;

pub mod archive;

/// 读取并解析一个 JSON 文件；失败时错误信息带完整文件路径。
fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取数据文件 {} 失败", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("解析数据文件 {} 失败", path.display()))
}

/// 解析并加载 `resources/` 内的一个 JSON 文件。
fn load_resource_json<T: DeserializeOwned>(app_paths: &AppPaths, relative_path: &str) -> Result<T> {
    let path = app_paths.resolve_resource_file(relative_path)?;
    load_json(&path)
}

/// 各领域的运行时数据，启动时加载并只读共享。
pub struct AppData {
    archives: archive::Database,
}

impl AppData {
    pub fn load(app_paths: &AppPaths) -> Result<Self> {
        let archives = archive::Database::load(app_paths)?;
        info!(
            "已加载档案目录（{} 个档案条目）",
            archives.catalog().archives.len()
        );
        Ok(Self { archives })
    }

    pub fn archives(&self) -> &archive::Database {
        &self.archives
    }
}
