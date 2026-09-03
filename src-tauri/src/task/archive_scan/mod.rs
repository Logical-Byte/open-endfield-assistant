//! 扫描档案库任务。
//!
//! 提供完整扫描功能：[`ArchiveScanTask`] 导航并扫描档案库全部 6 个子分类
//! （引号键触发）。

mod constants;
pub mod correction;
mod plan;
pub mod result;
mod scan_loop;
pub mod task;

pub use correction::{Corrected, CorrectionOverride, correct};
pub use result::{ScanReporter, ScanResult};
pub use task::ArchiveScanTask;
