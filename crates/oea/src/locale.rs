//! 应用语言的共享解析规则。游戏语言始终保持简体中文。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use oea_settings::UiLocale;

/// 自动化任务与识别证据的游戏语言语境：
/// - `zh-CN`：简体中文，当前唯一支持的游戏语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "locale/")]
pub enum GameLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
}
