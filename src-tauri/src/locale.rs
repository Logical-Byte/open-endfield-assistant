//! 应用语言的共享解析规则。游戏语言始终保持简体中文。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings/")]
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
    /// 当前用户系统界面语言，读取失败时使用英文。
    pub fn system_default() -> Self {
        crate::platform::locale::user_interface_language()
            .as_deref()
            .map(Self::from_language_tag)
            .unwrap_or(Self::EnUs)
    }
}
impl Default for UiLocale {
    fn default() -> Self {
        Self::system_default()
    }
}
