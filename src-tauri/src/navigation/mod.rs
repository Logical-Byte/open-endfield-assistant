//! 导航系统模块。
//!
//! - **场景**：`scenes`（界面标识、识别规则和跳转声明）；
//! - **导航**：[`Navigator`]（检测、规划与执行）；
//! - **跳转操作**：[`transition`]（有序跳转操作）。

pub mod navigator;
mod route_executor;
mod route_planner;
mod scene_detector;
pub mod scenes;
pub mod transition;

pub use navigator::Navigator;
