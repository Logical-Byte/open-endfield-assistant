//! 背包基质扫描任务，逐条发布识别结果与保留建议。

mod layout;
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
    pub image: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum WorkerType {
    Production,
    Simulation,
}
