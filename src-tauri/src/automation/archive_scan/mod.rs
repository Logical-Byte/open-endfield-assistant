//! 档案库扫描工作流与真实游戏工作者。
//!
//! [`workflow::ArchiveScanner`] 导航并扫描档案库全部 6 个子分类
//! （引号键触发）。

mod constants;
mod correction;
mod plan;
mod reporting;
mod scan_loop;
pub(crate) mod worker;
mod workflow;

pub(crate) use reporting::ScanResult;
