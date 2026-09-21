//! 六个档案库子界面的扫描顺序和数据分类映射。

use crate::navigation::{ArchiveSubscene, CentralPage, RecordsPage};

/// 六个档案库子界面的完整扫描顺序。
pub const SCAN_PLAN: &[ArchiveSubscene] = &[
    ArchiveSubscene::Media,
    ArchiveSubscene::Records(RecordsPage::Paper),
    ArchiveSubscene::Records(RecordsPage::Digital),
    ArchiveSubscene::Records(RecordsPage::Collection),
    ArchiveSubscene::Central(CentralPage::Archive),
    ArchiveSubscene::Central(CentralPage::Report),
];

/// 子界面所属的档案库大类 ID（`pageType`：`multi_media` / `text` / `document`）。
pub fn page_type_of(subscene: ArchiveSubscene) -> &'static str {
    match subscene {
        ArchiveSubscene::Media => "multi_media",
        ArchiveSubscene::Records(_) => "text",
        ArchiveSubscene::Central(_) => "document",
    }
}

/// 子界面所属的小类 ID（`categoryId`，与 `prts.json` 中 `allItems` 的
/// `categoryId` 一致）。
pub fn category_id_of(subscene: ArchiveSubscene) -> &'static str {
    match subscene {
        ArchiveSubscene::Media => "media",
        ArchiveSubscene::Records(RecordsPage::Paper) => "paper",
        ArchiveSubscene::Records(RecordsPage::Digital) => "digital",
        ArchiveSubscene::Records(RecordsPage::Collection) => "collection",
        ArchiveSubscene::Central(CentralPage::Archive) => "document",
        ArchiveSubscene::Central(CentralPage::Report) => "report",
    }
}
