//! 扫描档案库任务。
//!
//! 提供两类功能：
//! 1. [`scan_single_archive_detail`]：单次扫描当前档案详情（分号键触发，仅截屏识别）
//! 2. [`ArchiveScanTask`]：完整扫描档案库（引号键触发，导航并扫描全部 6 个子分类）

mod constants;
mod scan_loop;
pub mod scenes;
mod single_scan;
pub mod task;

pub use single_scan::scan_single_archive_detail;
pub use task::ArchiveScanTask;
