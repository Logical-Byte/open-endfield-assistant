//! 应用语言的共享解析规则。游戏语言始终保持简体中文。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// OEA 应用界面与原生提示使用的语言：
/// - `zh-CN`：简体中文。
/// - `en-US`：英文。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "locale/")]
pub enum UiLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}
impl UiLocale {
    /// 中文系统界面语言统一使用简体中文，其余语言使用英文。
    pub fn from_language_tag(tag: &str) -> Self {
        if tag
            .split(['-', '_'])
            .next()
            .is_some_and(|language| language.eq_ignore_ascii_case("zh"))
        {
            Self::ZhCn
        } else {
            Self::EnUs
        }
    }
}

/// 自动化任务与识别证据的游戏语言语境：
/// - `zh-CN`：简体中文，当前唯一支持的游戏语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "locale/")]
pub enum GameLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
}
