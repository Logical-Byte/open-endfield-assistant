use serde::Serialize;
use ts_rs::TS;

use crate::{automation, data::essence};

/// 屏幕上的一项词条。保留显示顺序，`kind` 表达语义类别，未知值不会补成默认等级。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceTermObservation")]
pub(crate) struct TermObservation {
    pub term_id: Option<essence::TermId>,
    pub kind: Option<essence::TermKind>,
    pub level: Option<u8>,
}

/// EER `essence_rarity.py` 可区分五星、四星与其他颜色，黑色/无效采样另记为未知。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceRarity")]
pub(crate) enum Rarity {
    Five,
    Four,
    Other,
    Unknown,
}

/// 识别不完整的字段，由前端翻译展示。词条位置从 1 开始，指屏幕显示顺序。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceRecognitionIssue")]
pub(crate) enum RecognitionIssue {
    Term { position: u8 },
    Level { position: u8 },
    TermLayout,
    Rarity,
    Locked,
    Abandoned,
}

/// 一枚物理基质的观察记录。`recordId` 仅在本次运行内稳定，相同内容不会合并。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceScanRecord")]
pub(crate) struct ScanRecord {
    pub run_id: u32,
    pub record_id: u32,
    /// 从背包顶部起算的绝对行、列，从 1 开始。
    pub row: u32,
    pub column: u32,
    pub terms: [TermObservation; 3],
    pub rarity: Rarity,
    pub locked: Option<bool>,
    pub abandoned: Option<bool>,
    pub issues: Vec<RecognitionIssue>,
    /// 只包含详情面板，避免保存 UID 和完整游戏画面。
    pub image: String,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceScanPhase")]
pub(crate) enum Phase {
    Preparing,
    Scanning,
    Scrolling,
    Finished,
}

/// 到达列表末尾、用户停止、扫描失败分别表达。完整遍历不保证每个字段均已识别。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceScanOutcome")]
pub(crate) enum Outcome {
    Completed,
    Stopped,
    Failed { error: automation::WorkerError },
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceScanProgress")]
pub(crate) struct Progress {
    pub run_id: u32,
    pub worker_type: automation::WorkerType,
    pub phase: Phase,
    pub scanned_count: u32,
    pub incomplete_count: u32,
    pub outcome: Option<Outcome>,
}

/// 最近一次基质扫描，仅保存在应用会话中。范围是启动时显示的基质列表。
/// 用户需先清除游戏筛选，扫描器无法证明被筛选隐藏的物品也已遍历。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essenceScan/", rename = "EssenceScanReport")]
pub(crate) struct Report {
    pub progress: Progress,
    pub records: Vec<ScanRecord>,
}
