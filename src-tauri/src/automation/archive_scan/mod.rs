//! 档案扫描任务。
//!
//! 封装档案扫描工作流、逐条结果上报和 [`Worker`](super::runtime::Worker) adapter。

mod constants;
mod correction;
mod plan;
mod reporting;
mod scan_loop;
mod worker;
mod workflow;

pub(crate) use reporting::ScanResult;
pub(crate) use worker::ArchiveScanWorker;
