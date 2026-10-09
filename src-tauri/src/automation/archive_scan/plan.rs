//! 六个档案库子界面的扫描顺序和数据分类映射。

use crate::data::archive;
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
pub fn page_type_of(subscene: ArchiveSubscene) -> archive::Page {
    match subscene {
        ArchiveSubscene::Media => archive::Page::MultiMedia,
        ArchiveSubscene::Records(_) => archive::Page::Text,
        ArchiveSubscene::Central(_) => archive::Page::Document,
    }
}

/// 子界面所属的小类 ID（`categoryId`，与 `prts.json` 中 `allItems` 的
/// `categoryId` 一致）。
pub fn category_id_of(subscene: ArchiveSubscene) -> archive::Category {
    match subscene {
        ArchiveSubscene::Media => archive::Category::Media,
        ArchiveSubscene::Records(RecordsPage::Paper) => archive::Category::Paper,
        ArchiveSubscene::Records(RecordsPage::Digital) => archive::Category::Digital,
        ArchiveSubscene::Records(RecordsPage::Collection) => archive::Category::Collection,
        ArchiveSubscene::Central(CentralPage::Archive) => archive::Category::Document,
        ArchiveSubscene::Central(CentralPage::Report) => archive::Category::Report,
    }
}
