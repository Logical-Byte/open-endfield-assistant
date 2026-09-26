//! 导航使用的具体 UI 状态。

use std::fmt;

/// 导航能够明确识别的游戏界面状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum UiState {
    /// 大世界
    Overworld,
    /// 协议终端
    Terminal,
    /// 档案库主界面、六个档案库子界面或档案详情页面。
    Archive(ArchiveState),
}

impl UiState {
    /// 当前能够明确识别的全部具体 UI 状态。
    #[cfg(test)]
    pub(crate) const ALL: [Self; 10] = [
        Self::Overworld,
        Self::Terminal,
        Self::Archive(ArchiveState::Main),
        Self::Archive(ArchiveState::Detail),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Media)),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
            RecordsPage::Paper,
        ))),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
            RecordsPage::Digital,
        ))),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
            RecordsPage::Collection,
        ))),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Central(
            CentralPage::Archive,
        ))),
        Self::Archive(ArchiveState::Subscene(ArchiveSubscene::Central(
            CentralPage::Report,
        ))),
    ];

    /// 构造具体 UI 状态“档案详情页面”。
    pub(crate) const fn archive_detail() -> Self {
        Self::Archive(ArchiveState::Detail)
    }

    /// 构造指定的档案库子界面具体 UI 状态。
    pub(crate) const fn archive_subscene(subscene: ArchiveSubscene) -> Self {
        Self::Archive(ArchiveState::Subscene(subscene))
    }
}

/// 档案库相关界面的具体状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArchiveState {
    /// 档案库主界面
    Main,
    /// 档案详情页面
    Detail,
    /// 音像存档、见闻辑录和中枢档案分类下的六个档案库子界面。
    Subscene(ArchiveSubscene),
}

/// 档案库子界面的具体分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArchiveSubscene {
    /// 音像存档 - 多媒体
    Media,
    /// 「见闻辑录 - 纸质记录」「见闻辑录 - 电子档案」或「见闻辑录 - 藏品」。
    Records(RecordsPage),
    /// 「中枢档案 - 中枢档案」或「中枢档案 - 调查报告」。
    Central(CentralPage),
}

impl fmt::Display for ArchiveSubscene {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Media => "音像存档 - 多媒体",
            Self::Records(RecordsPage::Paper) => "见闻辑录 - 纸质记录",
            Self::Records(RecordsPage::Digital) => "见闻辑录 - 电子档案",
            Self::Records(RecordsPage::Collection) => "见闻辑录 - 藏品",
            Self::Central(CentralPage::Archive) => "中枢档案 - 中枢档案",
            Self::Central(CentralPage::Report) => "中枢档案 - 调查报告",
        })
    }
}

/// 见闻辑录子界面的具体页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RecordsPage {
    /// 见闻辑录 - 纸质记录
    Paper,
    /// 见闻辑录 - 电子档案
    Digital,
    /// 见闻辑录 - 藏品
    Collection,
}

/// 中枢档案子界面的具体页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CentralPage {
    /// 中枢档案 - 中枢档案
    Archive,
    /// 中枢档案 - 调查报告
    Report,
}
