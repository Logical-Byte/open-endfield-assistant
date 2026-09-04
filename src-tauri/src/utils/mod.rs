//! 通用工具（基础设施，通用库）。
//!
//! - [`point`]：二维点 `Point2D`（含 az 泛型转换）；
//! - [`region`]：矩形区域 `Region2D`（720p 基准 ROI 的核心类型）；
//! - [`timeit`]：计时工具。

pub mod point;
pub mod region;
pub mod timeit;
