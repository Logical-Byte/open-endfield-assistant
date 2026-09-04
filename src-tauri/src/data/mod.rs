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

pub(crate) mod archive_title_index;
pub(crate) mod schema;

pub use archive_title_index::ArchiveTitleIndex;
pub use schema::{ArchiveContract, PrtsData};

/// 运行时加载的静态数据（不可变，跨线程只读共享）。
pub struct AppData {
    /// prts.json 完整数据（供前端查询分类中文名 / 自动补全候选，并构建档案标题索引）
    prts: PrtsData,
    /// 档案获取契约（archive_contract.json，供前端按档案 id 查询获取方式）
    archive_contract: ArchiveContract,
    /// 档案标题索引（启动时从 prts.json 派生构建一次）
    archive_titles: ArchiveTitleIndex,
}

impl AppData {
    /// 从 `resources/data/` 加载全部静态数据文件（新增数据文件在此登记）。
    pub fn load(app_paths: &AppPaths) -> Result<Self> {
        let data_dir = app_paths.resources_dir().join("data");

        let prts = Self::load_json::<PrtsData>(&data_dir.join("prts.json"))?;
        let archive_titles = ArchiveTitleIndex::from_prts(&prts);
        info!("已加载 prts.json（{} 个档案条目）", archive_titles.len());

        let archive_contract =
            Self::load_json::<ArchiveContract>(&data_dir.join("archive_contract.json"))?;
        let row_count: usize = archive_contract.categories.values().map(Vec::len).sum();
        info!("已加载 archive_contract.json（{} 条获取契约）", row_count);

        Ok(Self {
            prts,
            archive_contract,
            archive_titles,
        })
    }

    /// prts.json 完整数据。
    pub fn prts(&self) -> &PrtsData {
        &self.prts
    }

    /// 档案获取契约完整数据。
    pub fn archive_contract(&self) -> &ArchiveContract {
        &self.archive_contract
    }

    /// 档案标题索引。
    pub fn archive_titles(&self) -> &ArchiveTitleIndex {
        &self.archive_titles
    }

    /// 读取并解析一个 JSON 数据文件；失败时错误信息带完整文件路径。
    fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("读取数据文件 {} 失败", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("解析数据文件 {} 失败", path.display()))
    }
}
