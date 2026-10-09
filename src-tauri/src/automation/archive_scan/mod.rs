//! 档案扫描任务。
//!
//! 封装档案扫描工作流、逐条结果上报和 [`Worker`](super::runtime::Worker) adapter。

use ts_rs::TS;

mod constants;
mod correction;
mod plan;
mod reporting;
mod scan_loop;
mod simulation_worker;
mod worker;
mod workflow;

#[cfg(feature = "cli")]
pub(crate) use constants::OCR_ROI;

pub(crate) use reporting::ScannedItem;
pub(crate) use simulation_worker::SimulatedArchiveScanWorker;
pub(crate) use worker::ArchiveScanWorker;

#[derive(serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "archiveScan/")]
pub(crate) enum WorkerType {
    Production,
    Simulation,
}
