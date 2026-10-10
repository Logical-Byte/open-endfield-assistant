//! 档案扫描任务。
//!
//! 封装档案扫描工作流、逐条结果上报和 [`Worker`](super::runtime::Worker) adapter。

use ts_rs::TS;

mod ocr_correction;
mod reporting;
mod simulation_worker;
mod worker;

#[cfg(feature = "cli")]
pub(crate) use worker::OCR_ROI;

pub(crate) use reporting::ScannedItem;
pub(crate) use simulation_worker::SimulationWorker;
pub(crate) use worker::Worker;

#[derive(serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) enum WorkerType {
    Production,
    Simulation,
}
