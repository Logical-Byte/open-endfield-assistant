//! 从百趣演武活动页面抽取铭牌，直到命中用户选择的停止条件。

mod lucky;
mod model;
mod recognition;
mod reporting;
mod simulation_worker;
mod worker;

pub(crate) use lucky::{DailyLucky, daily};
pub(crate) use model::{Conditions, MatchedHand};
pub(crate) use simulation_worker::SimulationWorker;
pub(crate) use worker::{Progress, Worker};
