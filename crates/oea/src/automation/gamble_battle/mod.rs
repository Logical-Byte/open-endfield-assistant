//! 从百趣演武活动页面抽取铭牌，直到命中用户选择的停止条件。

mod lucky;
mod model;
mod recognition;
mod reporting;
mod simulation_worker;
mod worker;

#[cfg(feature = "legacy-tauri")]
pub(crate) use lucky::DailyLucky;
pub(crate) use lucky::daily;
pub(crate) use model::{Conditions, MatchedHand};
#[cfg(feature = "desktop")]
pub(crate) use model::{HandKind, Suit};
pub(crate) use simulation_worker::SimulationWorker;
pub(crate) use worker::{Progress, Worker};
