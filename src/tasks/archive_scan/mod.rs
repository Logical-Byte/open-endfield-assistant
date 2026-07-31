//! 扫描档案库任务。
//!
//! 从任意受支持的界面出发，导航到档案库主界面，遍历全部 6 个子分类，
//! 扫描每个子分类中的所有档案，OCR 识别档案标题并记录 SUCCESS 日志。

mod scan_loop;
pub mod scenes;
pub mod task;

pub use task::ArchiveScanTask;
