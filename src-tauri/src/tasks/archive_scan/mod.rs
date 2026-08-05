//! 扫描档案库任务。
//!
//! 提供两类功能：
//! 1. [`scan_single_archive_detail`]：单次扫描当前档案详情（分号键触发，仅截屏识别）
//! 2. [`ArchiveScanTask`]：完整扫描档案库（引号键触发，导航并扫描全部 6 个子分类）

mod constants;
mod scan_loop;
mod single_scan;
pub mod task;

pub use single_scan::scan_single_archive_detail;
pub use task::ArchiveScanTask;

/// 记录一条 SUCCESS 级别的日志（基于 tracing INFO 级别，但标记为 SUCCESS）。
///
/// 业务语义（扫描到档案标题等关键结果）归任务层，不属于通用日志库。
/// 通过 `#[macro_export]` 在 crate 根可见，调用点无需改动。
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
        tracing::info!("SUCCESS: {}", format!($($arg)*))
    };
}
