//! `prts.json` 的 Rust 类型定义（与前端 `src/lib/prts.ts` 对齐）。
//!
//! prts.json 由四部分（PrtsPage / PrtsCategory / firstLv / allItems）组成，
//! 构成一棵四级树形结构，从页面逐级下钻到具体档案条目：
//!
//!   PrtsPage（页面，键 = pageType）
//!     └─ categoryIds → PrtsCategory（分类，键 = categoryId）
//!          └─ firstLvIds → PrtsFirstLv（一级条目，键 = firstLvId）
//!               └─ itemIds → PrtsAllItem（具体条目，键 = id）
//!
//! 各部分的键均为对应实体的 id（Record 键），值中通过 id 数组（categoryIds /
//! firstLvIds / itemIds）表达父子关系；同时子实体（category / firstLv / item）
//! 反向持有父级 id（categoryId / firstLvId），便于向上回溯。
//!
//! 四个部分的 Record 写入顺序即前端展示顺序（见 `scripts/tasks/makePrts.ts`），
//! 故使用 [`IndexMap`] 保持顺序（与前端 `Object.values` 遍历顺序一致）。

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// 档案库页面类型（音像存档 / 见闻辑录 / 中枢档案）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrtsPageType {
    /// 音像存档
    MultiMedia,
    /// 见闻辑录
    Text,
    /// 中枢档案
    Document,
}

/// 档案库页面（音像存档 / 见闻辑录 / 中枢档案）。
/// PrtsData.PrtsPage 以 pageType 为键。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrtsPage {
    /// 页面名称（中文，取自 i18n 文本表）
    pub name: String,
    /// 页面类型，同时作为本页面的唯一标识（Record 键）
    pub page_type: PrtsPageType,
    /// 该页面下的分类 id 列表（按所属页面、分类 order 排序）
    pub category_ids: Vec<String>,
}

/// 档案库分类（藏品、电子档案、纸质记录……）。
/// PrtsData.PrtsCategory 以 categoryId 为键。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrtsCategory {
    /// 分类唯一标识（Record 键）
    pub category_id: String,
    /// 分类名称（中文）
    pub name: String,
    /// 分类在所属页面内的展示顺序（数字越小越靠前）
    pub order: i64,
    /// 所属页面类型
    pub r#type: PrtsPageType,
    /// 该分类下的一级条目 id 列表（按一级条目 order 排序）
    pub first_lv_ids: Vec<String>,
}

/// 一级条目（页面 → 分类下的中间层级）。
/// PrtsData.firstLv 以 firstLvId 为键。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrtsFirstLv {
    /// 所属分类 id（反向引用 PrtsCategory.categoryId）
    pub category_id: String,
    /// 一级条目唯一标识（Record 键）
    pub first_lv_id: String,
    /// 该一级条目下的具体档案条目 id 列表（保持数据表原始顺序）
    pub item_ids: Vec<String>,
    /// 一级条目名称（中文）
    pub name: String,
    /// 一级条目在所属分类内的展示顺序（数字越小越靠前）
    pub order: i64,
    /// 所属页面类型（与所属分类的 type 保持一致）
    pub r#type: PrtsPageType,
}

/// 具体档案条目（树形结构的叶子节点）。
/// PrtsData.allItems 以 id 为键。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrtsAllItem {
    /// 所属分类 id（由所属一级条目的 categoryId 推导）
    pub category_id: String,
    /// 所属一级条目 id（反向引用 PrtsFirstLv.firstLvId）
    pub first_lv_id: String,
    /// 档案条目唯一标识（Record 键）
    pub id: String,
    /// 条目名称（中文）
    pub name: String,
    /// 条目在所属一级条目内的展示顺序（数字越小越靠前）
    pub order: i64,
    /// 展示标题（中文）：音像存档（multi_media）与名称一致；
    /// 文档 / 文本则以 contentId 在富文本表中查找，查不到时回退为名称。
    pub title: String,
    /// 条目所属页面类型
    pub r#type: PrtsPageType,
}

/// prts.json 完整结构。
///
/// 四个部分均为「以 id 为键的 Record」，值中通过 id 数组表达层级关系；
/// 使用 [`IndexMap`] 保留 JSON 中的写入顺序（即前端展示顺序，
/// 见 `scripts/tasks/makePrts.ts` 的排序逻辑）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrtsData {
    /// 档案库页面（键 = pageType，按 音像存档 → 见闻辑录 → 中枢档案 排序）
    #[serde(rename = "PrtsPage")]
    pub prts_page: IndexMap<String, PrtsPage>,
    /// 档案库分类（键 = categoryId，按 页面、分类 order 排序）
    #[serde(rename = "PrtsCategory")]
    pub prts_category: IndexMap<String, PrtsCategory>,
    /// 一级条目（键 = firstLvId，按 页面、分类 order、一级条目 order 排序）
    #[serde(rename = "firstLv")]
    pub first_lv: IndexMap<String, PrtsFirstLv>,
    /// 具体档案条目（键 = id，按 页面、分类 order、一级条目 order、条目 order 排序）
    #[serde(rename = "allItems")]
    pub all_items: IndexMap<String, PrtsAllItem>,
}
