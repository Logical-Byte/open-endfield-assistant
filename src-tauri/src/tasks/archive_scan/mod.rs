//! 扫描档案库任务。
//!
//! 提供两类功能：
//! 1. [`single_scan`]：单次扫描当前档案详情（分号键触发，仅截屏识别）
//! 2. [`ArchiveScanTask`]：完整扫描档案库（引号键触发，导航并扫描全部 6 个子分类）

mod constants;
pub mod correction;
mod plan;
pub mod result;
mod scan_loop;
mod single_scan;
pub mod task;

pub use correction::{Corrected, CorrectionIndex};
pub use result::{ScanReporter, ScanResult};
pub use single_scan::single_scan;
pub use task::ArchiveScanTask;
