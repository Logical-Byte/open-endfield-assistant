//! 背包基质扫描任务，逐条发布识别结果与保留建议。

mod layout;
mod marking;
mod reporting;
mod simulation_worker;
#[cfg(test)]
mod tests;
mod worker;
mod workflow;

use crate::essence;

pub(crate) use simulation_worker::SimulatedEssenceScanWorker;
pub(crate) use worker::EssenceScanWorker;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScannedItem {
    /// 序号与位置均从 1 开始，按行优先遍历。
    pub sequence: u32,
    pub page: u32,
    pub row: u32,
    pub column: u32,
    pub essence: essence::Essence,
    pub evaluation: essence::Evaluation,
    pub marking: Marking,
    pub image: Option<String>,
}

/// 标记结果与扫描时的基质状态分开保存，便于确认实际发生的操作。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum Marking {
    Disabled,
    Skipped,
    AlreadySet {
        action: essence::MarkAction,
    },
    Applied {
        action: essence::MarkAction,
    },
    Simulated {
        action: essence::MarkAction,
    },
    Failed {
        action: essence::MarkAction,
        error: String,
    },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum WorkerType {
    Production,
    Simulation,
}
