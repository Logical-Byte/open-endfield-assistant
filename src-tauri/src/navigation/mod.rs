//! 游戏 UI 状态识别与导航。
//!
//! `recognition` 通过固定状态组树从同一张截图得到具体 [`UiState`]；`graph` 声明每个
//! 状态可执行的符号操作和全部可能结果；`policy` 从目的状态反向生成强策略；
//! `executor` 解释动作并在每次观测后继续查询同一份策略。档案 OCR、内容翻页和扫描
//! 顺序属于 `task::archive_scan`，不进入通用导航模型。

mod executor;
mod graph;
mod navigator;
mod policy;
mod recognition;
mod state;
mod targets;
mod transition;

pub(crate) use navigator::Navigator;
pub(crate) use state::{ArchiveState, ArchiveSubscene, CentralPage, RecordsPage, UiState};
