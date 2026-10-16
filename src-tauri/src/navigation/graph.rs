//! 验证并保存静态声明的 UI 状态跳转图。
//!
//! 图节点只包含具体 UI 状态。每条 `transition` 的 `outcomes` 描述动作成功后所有合法
//! 观测结果；动作没有生效时仍停留在来源状态属于 `executor` 处理的失败，不为每条边
//! 重复声明自环。只改变详情内容、不改变具体 UI 状态的扫描动作也不进入本图。

use std::collections::{HashMap, HashSet};

use crate::automation::{Key, Point720p};

use super::{
    state::{ArchiveState, ArchiveSubscene, CentralPage, RecordsPage, UiState},
    targets::{
        ARCHIVE_CENTRAL_ENTRY, ARCHIVE_DETAIL_CLOSE, ARCHIVE_MAIN_CLOSE, ARCHIVE_MEDIA_ENTRY,
        ARCHIVE_RECORDS_ENTRY, ARCHIVE_SUBSCENE_CLOSE, TERMINAL_ARCHIVE_ENTRY,
    },
    transition::{Op, Transition},
};

/// 具体 UI 状态“档案库主界面”。
const ARCHIVE_MAIN: UiState = UiState::Archive(ArchiveState::Main);
/// 具体 UI 状态“档案详情页面”。
const ARCHIVE_DETAIL: UiState = UiState::Archive(ArchiveState::Detail);
/// 具体 UI 状态“音像存档 - 多媒体”。
const MULTIMEDIA: UiState = UiState::Archive(ArchiveState::Subscene(ArchiveSubscene::Media));
/// 具体 UI 状态“见闻辑录 - 纸质记录”。
const PAPER: UiState = UiState::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
    RecordsPage::Paper,
)));
/// 具体 UI 状态“见闻辑录 - 电子档案”。
const ELECTRONIC: UiState = UiState::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
    RecordsPage::Digital,
)));
/// 具体 UI 状态“见闻辑录 - 藏品”。
const COLLECTION: UiState = UiState::Archive(ArchiveState::Subscene(ArchiveSubscene::Records(
    RecordsPage::Collection,
)));
/// 具体 UI 状态“中枢档案 - 中枢档案”。
const CENTRAL_ARCHIVES: UiState = UiState::Archive(ArchiveState::Subscene(
    ArchiveSubscene::Central(CentralPage::Archive),
));
/// 具体 UI 状态“中枢档案 - 调查报告”。
const INVESTIGATION_REPORT: UiState = UiState::Archive(ArchiveState::Subscene(
    ArchiveSubscene::Central(CentralPage::Report),
));

/// 关闭任意档案库子界面，结果为“档案库主界面”。
const CLOSE_SUBSCENE: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_SUBSCENE_CLOSE,
        fallback: None,
    }],
    outcomes: &[ARCHIVE_MAIN],
};
/// 在任意档案库子界面打开当前条目，结果为“档案详情页面”。
const OPEN_DETAIL: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 401, y: 182 })],
    outcomes: &[ARCHIVE_DETAIL],
};
/// 在见闻辑录子界面切换到“见闻辑录 - 纸质记录”。
const TO_PAPER: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 210, y: 138 })],
    outcomes: &[PAPER],
};
/// 在见闻辑录子界面切换到“见闻辑录 - 电子档案”。
const TO_ELECTRONIC: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 210, y: 202 })],
    outcomes: &[ELECTRONIC],
};
/// 在见闻辑录子界面切换到“见闻辑录 - 藏品”。
const TO_COLLECTION: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 210, y: 266 })],
    outcomes: &[COLLECTION],
};
/// 在中枢档案子界面切换到“中枢档案 - 中枢档案”。
const TO_CENTRAL_ARCHIVES: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 210, y: 138 })],
    outcomes: &[CENTRAL_ARCHIVES],
};
/// 在中枢档案子界面切换到“中枢档案 - 调查报告”。
const TO_INVESTIGATION_REPORT: Transition<'static> = Transition {
    ops: &[Op::Click(Point720p { x: 210, y: 202 })],
    outcomes: &[INVESTIGATION_REPORT],
};
/// 从“协议终端”打开档案库，结果为“档案库主界面”。
const TERMINAL_TO_MAIN: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: TERMINAL_ARCHIVE_ENTRY,
        fallback: None,
    }],
    outcomes: &[ARCHIVE_MAIN],
};
/// 关闭“档案详情页面”，回到打开它的六个档案库子界面之一。
const DETAIL_TO_SUBSCENE: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_DETAIL_CLOSE,
        fallback: Some(Point720p { x: 1240, y: 50 }),
    }],
    outcomes: &[
        MULTIMEDIA,
        PAPER,
        ELECTRONIC,
        COLLECTION,
        CENTRAL_ARCHIVES,
        INVESTIGATION_REPORT,
    ],
};
/// 关闭“档案库主界面”，结果为“协议终端”。
const MAIN_TO_TERMINAL: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_MAIN_CLOSE,
        fallback: None,
    }],
    outcomes: &[UiState::Terminal],
};
/// 从“档案库主界面”打开“音像存档 - 多媒体”。
const MAIN_TO_MULTIMEDIA: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_MEDIA_ENTRY,
        fallback: None,
    }],
    outcomes: &[MULTIMEDIA],
};
/// 从“档案库主界面”打开见闻辑录，结果为“见闻辑录 - 纸质记录”。
const MAIN_TO_PAPER: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_RECORDS_ENTRY,
        fallback: None,
    }],
    outcomes: &[PAPER],
};
/// 从“档案库主界面”打开中枢档案，结果为“中枢档案 - 中枢档案”。
const MAIN_TO_CENTRAL_ARCHIVES: Transition<'static> = Transition {
    ops: &[Op::FindAndClickTemplate {
        target: ARCHIVE_CENTRAL_ENTRY,
        fallback: None,
    }],
    outcomes: &[CENTRAL_ARCHIVES],
};

/// 声明顺序稳定的 UI 状态跳转图。
///
/// 每个 `Vec` 的顺序是相同来源状态的 `transition` 声明顺序。强策略在最坏剩余动作数
/// 相同时依赖这个顺序作稳定选择，因此构造时绝不重排 `transition`。
pub(super) struct NavigationGraph {
    transitions: HashMap<UiState, Vec<Transition<'static>>>,
}

impl NavigationGraph {
    /// 构造游戏当前版本声明的完整导航图。
    pub(super) fn production() -> Self {
        let mut transitions = HashMap::new();
        transitions.insert(
            UiState::Overworld,
            vec![Transition {
                ops: &[Op::PressKey(Key::Escape)],
                outcomes: &[UiState::Terminal],
            }],
        );
        transitions.insert(UiState::Terminal, vec![TERMINAL_TO_MAIN]);
        transitions.insert(ARCHIVE_MAIN, main_transitions());
        for subscene in [
            ArchiveSubscene::Media,
            ArchiveSubscene::Records(RecordsPage::Paper),
            ArchiveSubscene::Records(RecordsPage::Digital),
            ArchiveSubscene::Records(RecordsPage::Collection),
            ArchiveSubscene::Central(CentralPage::Archive),
            ArchiveSubscene::Central(CentralPage::Report),
        ] {
            transitions.insert(
                UiState::archive_subscene(subscene),
                subscene_transitions(subscene),
            );
        }
        transitions.insert(ARCHIVE_DETAIL, vec![DETAIL_TO_SUBSCENE]);
        Self::new(transitions)
    }

    /// 构造并验证静态 `transition` 声明。
    ///
    /// # Panics
    ///
    /// 当一条 `transition` 没有 `outcomes`，或同一条 `transition` 重复声明某个
    /// `outcome` 时 `panic`。这些都是源码中的图声明错误，不是运行环境能恢复的故障。
    pub(super) fn new(transitions: HashMap<UiState, Vec<Transition<'static>>>) -> Self {
        for (source, source_transitions) in &transitions {
            for (transition_index, transition) in source_transitions.iter().enumerate() {
                assert!(
                    !transition.outcomes.is_empty(),
                    "导航图无效：来源状态 {source:?} 的第 {} 条跳转没有结果状态",
                    transition_index + 1,
                );

                let mut seen_outcomes = HashSet::new();
                for outcome in transition.outcomes {
                    assert!(
                        seen_outcomes.insert(*outcome),
                        "导航图无效：来源状态 {source:?} 的第 {} 条跳转重复声明结果状态 {outcome:?}",
                        transition_index + 1,
                    );
                }
            }
        }

        Self { transitions }
    }

    pub(super) fn sources(&self) -> impl Iterator<Item = (UiState, &[Transition<'static>])> + '_ {
        self.transitions
            .iter()
            .map(|(source, transitions)| (*source, transitions.as_slice()))
    }
}

fn main_transitions() -> Vec<Transition<'static>> {
    vec![
        MAIN_TO_TERMINAL,
        MAIN_TO_MULTIMEDIA,
        MAIN_TO_PAPER,
        MAIN_TO_CENTRAL_ARCHIVES,
    ]
}

fn subscene_transitions(subscene: ArchiveSubscene) -> Vec<Transition<'static>> {
    match subscene {
        ArchiveSubscene::Media => vec![CLOSE_SUBSCENE, OPEN_DETAIL],
        ArchiveSubscene::Records(RecordsPage::Paper) => {
            vec![CLOSE_SUBSCENE, OPEN_DETAIL, TO_ELECTRONIC, TO_COLLECTION]
        }
        ArchiveSubscene::Records(RecordsPage::Digital) => {
            vec![CLOSE_SUBSCENE, OPEN_DETAIL, TO_PAPER, TO_COLLECTION]
        }
        ArchiveSubscene::Records(RecordsPage::Collection) => {
            vec![CLOSE_SUBSCENE, OPEN_DETAIL, TO_PAPER, TO_ELECTRONIC]
        }
        ArchiveSubscene::Central(CentralPage::Archive) => {
            vec![CLOSE_SUBSCENE, OPEN_DETAIL, TO_INVESTIGATION_REPORT]
        }
        ArchiveSubscene::Central(CentralPage::Report) => {
            vec![CLOSE_SUBSCENE, OPEN_DETAIL, TO_CENTRAL_ARCHIVES]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::NavigationGraph;
    use crate::navigation::{state::UiState, transition::Transition};

    /// 当前测试强制要求能从每个具体 UI 状态到达的目的状态。
    const SUPPORTED_DESTINATIONS: [UiState; 9] = [
        UiState::Terminal,
        super::ARCHIVE_MAIN,
        super::ARCHIVE_DETAIL,
        super::MULTIMEDIA,
        super::PAPER,
        super::ELECTRONIC,
        super::COLLECTION,
        super::CENTRAL_ARCHIVES,
        super::INVESTIGATION_REPORT,
    ];

    #[test]
    #[should_panic(expected = "来源状态 Overworld 的第 1 条跳转没有结果状态")]
    fn rejects_transition_without_outcomes() {
        let _ = NavigationGraph::new(HashMap::from([(
            UiState::Overworld,
            vec![Transition {
                ops: &[],
                outcomes: &[],
            }],
        )]));
    }

    #[test]
    #[should_panic(expected = "来源状态 Overworld 的第 1 条跳转重复声明结果状态 Terminal")]
    fn rejects_transition_with_duplicate_outcomes() {
        let _ = NavigationGraph::new(HashMap::from([(
            UiState::Overworld,
            vec![Transition {
                ops: &[],
                outcomes: &[UiState::Terminal, UiState::Terminal],
            }],
        )]));
    }

    #[test]
    fn every_supported_destination_has_a_policy_from_every_state() {
        let graph = NavigationGraph::production();

        for destination in SUPPORTED_DESTINATIONS {
            let policy = crate::navigation::policy::NavigationPolicy::build(
                &graph,
                [destination].into_iter(),
            )
            .unwrap();
            for source in UiState::ALL {
                assert!(
                    policy.covers(source),
                    "{source:?} 应能保证抵达 {destination:?}"
                );
            }
        }
    }
}
