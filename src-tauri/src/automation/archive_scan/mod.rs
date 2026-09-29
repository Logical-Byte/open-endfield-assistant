//! 档案扫描任务。
//!
//! 封装档案扫描工作流、逐条结果上报和 [`Worker`](super::runtime::Worker) adapter。

mod constants;
mod correction;
mod plan;
mod reporting;
mod scan_loop;
pub(crate) mod worker;
mod workflow;

pub(crate) use reporting::ScanResult;
