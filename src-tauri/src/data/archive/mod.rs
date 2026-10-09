//! 档案目录与查询。上游资源结构和索引组织方式均为私有实现。

mod database;
mod model;
mod source;
mod title_index;

pub(crate) use database::Candidates;
pub use database::Database;
pub use model::{
    AcquisitionMethod, ArchiveEntry, ArchiveId, Catalog, Category, CategoryEntry, Page, PageEntry,
};
#[cfg(test)]
pub(crate) use title_index::NORM_MAX_CHARS;
pub(crate) use title_index::normalize;
