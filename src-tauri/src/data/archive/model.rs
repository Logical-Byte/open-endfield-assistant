//! 前后端共享的精简档案目录契约。

use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// 一份具体档案的稳定身份，与标题及扫描记录 ID 无关。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub struct ArchiveId(#[ts(type = "string & { readonly __brand: 'ArchiveId' }")] String);

impl ArchiveId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArchiveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 档案库的大页面，与页面下的分类身份不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "archive/")]
pub enum Page {
    /// 音像存档。
    MultiMedia,
    /// 见闻辑录。
    Text,
    /// 中枢档案。
    Document,
}

/// 档案所属的小分类，用于限定标题匹配范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "archive/")]
pub enum Category {
    /// 多媒体。
    Media,
    /// 纸质记录。
    Paper,
    /// 电子档案。
    Digital,
    /// 藏品。
    Collection,
    /// 中枢档案。
    Document,
    /// 调查报告。
    Report,
}

/// 档案获取途径，不包含任务、地图点位等获取参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "archive/")]
pub enum AcquisitionMethod {
    /// 地图拾取。
    Map,
    /// 任务获取。
    Mission,
    /// 自动解锁。
    Auto,
    /// 商店购买。
    Shop,
    /// 研究提交，序列化名称沿用上游契约。
    #[serde(rename = "invstgt")]
    Investigate,
}

/// 资源携带的双语文本，当前游戏消费固定使用中文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub struct LocalizedText {
    #[serde(rename = "zh-CN")]
    pub zh_cn: String,
    #[serde(rename = "en-US")]
    pub en_us: String,
}

/// 扫描证据的游戏语言语境，当前只支持简体中文。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub enum GameLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
}

/// 页面目录条目。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub struct PageEntry {
    /// 页面身份，与名称和分类 ID 无关。
    pub id: Page,
    /// 页面显示名称。
    pub name: LocalizedText,
}

/// 分类目录条目。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub struct CategoryEntry {
    /// 分类身份。
    pub id: Category,
    /// 所属页面。
    pub page: Page,
    /// 分类显示名称。
    pub name: LocalizedText,
}

/// 具体档案条目，同标题的不同 ID 仍是不同档案。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "archive/")]
pub struct ArchiveEntry {
    /// 关联扫描证据与外部地图导出的稳定档案身份。
    pub id: ArchiveId,
    /// 匹配标题时使用的分类范围。
    pub category: Category,
    /// 档案详情的完整标题，用于展示和 OCR 候选匹配。
    pub title: LocalizedText,
    /// 获取途径，用于显示和地图拾取筛选。
    pub acquisition_method: AcquisitionMethod,
}

/// 前后端共享的只读档案目录，各数组顺序即展示及导出顺序。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "archive/")]
pub struct Catalog {
    /// 页面顺序。
    pub pages: Vec<PageEntry>,
    /// 分类顺序，包含所属页面信息。
    pub categories: Vec<CategoryEntry>,
    /// 全部档案，包含未扫描的条目。
    pub archives: Vec<ArchiveEntry>,
}
