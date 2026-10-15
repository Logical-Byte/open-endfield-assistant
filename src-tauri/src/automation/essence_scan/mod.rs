//! 基质背包扫描。记录观察事实，不评估规则、不修改锁定或弃用标记。

mod layout;
mod model;
mod recognition;
mod reporting;
mod worker;
mod workflow;

pub(crate) use model::{Progress, Report, ScanRecord};
pub(crate) use reporting::ReportStore;
pub(crate) use worker::Worker;

#[cfg(test)]
mod tests;
