//! 从多结果导航图反向生成强策略。
//!
//! 导航图中的每条 `transition` 是一条有向超边：它从一个来源状态指向一个
//! 非空的 `outcomes` 集合。执行者能选择超边，环境决定实际落入哪个 `outcome`，
//! 因此一条超边只有在全部 `outcomes` 都已能保证抵达目的地时才安全。
//!
//! # 构造步骤
//!
//! 1. 把所有目的状态放入覆盖集，它们的 `remaining_hops` 为零。
//! 2. 对每个未覆盖的来源状态，找出 `outcomes` 全部已在覆盖集中的超边。
//! 3. 每条可用超边的代价是 `1 + max(outcome.remaining_hops)`。选择代价最小的
//!    超边；代价相同时保留声明顺序中的第一条。
//! 4. 一轮扫描完成后才批量加入新状态。同一轮新发现的状态不能相互作为已有保证。
//! 5. 重复扩张，直到某一轮没有新状态。留在覆盖集外的状态不存在通往这组
//!    目的地的强策略，因此结果可以是 `partial policy`。
//!
//! # 正确性
//!
//! 对分层轮次做归纳。第零层是目的状态，不需要动作即可抵达。若某状态在后续
//! 轮次被加入，它选中超边的每个 `outcome` 都已在更早层中；根据归纳假设，
//! 环境无论选择哪个已声明结果，都有后续策略抵达目的地。新状态的最坏剩余步数比
//! 所有 `outcomes` 的最大值多一，所以任一已声明结果都会使该值严格下降。这证明了
//! 强可达性，也证明了模型内执行最多经过 `remaining_hops` 次动作就会到达目的地。
//!
//! 分层扩张还给出最小的最坏步数。对于非目的状态和 `k > 0`，它能在第 `k`
//! 层及以前保证到达目的地，当且仅当它存在一条超边，其全部 `outcomes` 的
//! 层数至多为 `k - 1`。算法逐轮检查的正是这个
//! 条件，并在同一来源的所有可用超边中最小化最大 `outcome` 层数。因此一个状态首次被
//! 覆盖时得到的 `remaining_hops` 就是所有强策略中的最小最坏步数。
//!
//! 图中的具体 UI 状态有限，每个非空轮次都至少加入一个之前未覆盖的状态，且永不移除，
//! 所以构造过程也一定终止。

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use super::{graph::NavigationGraph, state::UiState, transition::Transition};

/// 一个具体 UI 状态下应执行的策略步骤。
#[derive(Clone, Copy)]
pub(super) struct PolicyStep {
    /// 当前具体 UI 状态应选择的有向超边。
    pub(super) transition: Transition<'static>,
    /// 按该策略执行时，到达任一目的状态的最坏剩余动作数。
    pub(super) remaining_hops: u16,
}

/// 面向一个或多个目的状态的强策略。
pub(super) struct NavigationPolicy {
    destinations: HashSet<UiState>,
    steps: HashMap<UiState, PolicyStep>,
}

impl NavigationPolicy {
    /// 接受调用方的目的状态 `Iterator`；核心算法只处理收集后的具体集合。
    pub(super) fn build<I>(graph: &NavigationGraph, destinations: I) -> Result<Self>
    where
        I: Iterator<Item = UiState>,
    {
        Self::build_for_set(graph, destinations.collect())
    }

    /// 在具体目的状态集合上运行模块文档描述的反向分层超图搜索。
    fn build_for_set(graph: &NavigationGraph, destinations: HashSet<UiState>) -> Result<Self> {
        if destinations.is_empty() {
            bail!("导航策略至少需要一个目的状态");
        }

        let mut remaining_hops_by_state = destinations
            .iter()
            .copied()
            .map(|state| (state, 0_u16))
            .collect::<HashMap<_, _>>();
        let mut steps = HashMap::new();

        loop {
            let mut additions = Vec::new();
            for (source, transitions) in graph.sources() {
                if remaining_hops_by_state.contains_key(&source) {
                    continue;
                }

                let mut selected: Option<PolicyStep> = None;
                for transition in transitions {
                    let Some(outcome_hops) = transition
                        .outcomes
                        .iter()
                        .map(|outcome| remaining_hops_by_state.get(outcome).copied())
                        .collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };
                    let worst_outcome = outcome_hops
                        .into_iter()
                        .max()
                        .expect("导航图已保证 transition 至少有一个结果状态");
                    let remaining_hops = worst_outcome
                        .checked_add(1)
                        .ok_or_else(|| anyhow::anyhow!("导航策略剩余步数溢出"))?;
                    let candidate = PolicyStep {
                        transition: *transition,
                        remaining_hops,
                    };
                    if selected.is_none_or(|current| remaining_hops < current.remaining_hops) {
                        selected = Some(candidate);
                    }
                }

                if let Some(step) = selected {
                    additions.push((source, step));
                }
            }

            if additions.is_empty() {
                break;
            }
            // 批量提交本轮结果，避免同层状态在尚未建立归纳保证时相互依赖。
            for (source, step) in additions {
                remaining_hops_by_state.insert(source, step.remaining_hops);
                steps.insert(source, step);
            }
        }

        Ok(Self {
            destinations,
            steps,
        })
    }

    pub(super) fn is_destination(&self, state: UiState) -> bool {
        self.destinations.contains(&state)
    }

    pub(super) fn step(&self, state: UiState) -> Option<PolicyStep> {
        self.steps.get(&state).copied()
    }

    #[cfg(test)]
    pub(super) fn covers(&self, state: UiState) -> bool {
        self.is_destination(state) || self.steps.contains_key(&state)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::NavigationPolicy;
    use crate::navigation::{
        graph::NavigationGraph,
        state::{ArchiveState, UiState},
        transition::Transition,
    };

    /// 测试用跳转：大世界可能到达协议终端或档案库主界面。
    const WORLD_TO_BOTH: Transition<'static> = Transition {
        ops: &[],
        outcomes: &[UiState::Terminal, UiState::Archive(ArchiveState::Main)],
    };
    /// 测试用跳转：大世界到达协议终端。
    const WORLD_TO_TERMINAL: Transition<'static> = Transition {
        ops: &[],
        outcomes: &[UiState::Terminal],
    };
    /// 测试用跳转：大世界到达档案库主界面。
    const WORLD_TO_MAIN: Transition<'static> = Transition {
        ops: &[],
        outcomes: &[UiState::Archive(ArchiveState::Main)],
    };
    /// 测试用跳转：协议终端到达档案库主界面。
    const TERMINAL_TO_MAIN: Transition<'static> = Transition {
        ops: &[],
        outcomes: &[UiState::Archive(ArchiveState::Main)],
    };

    #[test]
    fn covers_a_source_only_after_every_outcome_is_covered() {
        let graph = NavigationGraph::new(HashMap::from([
            (UiState::Overworld, vec![WORLD_TO_BOTH]),
            (UiState::Terminal, vec![TERMINAL_TO_MAIN]),
        ]));

        let policy = NavigationPolicy::build(
            &graph,
            [UiState::Terminal, UiState::Archive(ArchiveState::Main)].into_iter(),
        )
        .unwrap();

        assert!(policy.covers(UiState::Overworld));
        assert_eq!(policy.step(UiState::Overworld).unwrap().remaining_hops, 1);
    }

    #[test]
    fn minimizes_the_worst_remaining_hops() {
        let graph = NavigationGraph::new(HashMap::from([
            (UiState::Overworld, vec![WORLD_TO_TERMINAL, WORLD_TO_MAIN]),
            (UiState::Terminal, vec![TERMINAL_TO_MAIN]),
        ]));

        let policy =
            NavigationPolicy::build(&graph, [UiState::Archive(ArchiveState::Main)].into_iter())
                .unwrap();

        let step = policy.step(UiState::Overworld).unwrap();
        assert_eq!(step.transition.outcomes, WORLD_TO_MAIN.outcomes);
        assert_eq!(step.remaining_hops, 1);
    }

    #[test]
    fn declaration_order_breaks_equal_cost_ties() {
        let graph = NavigationGraph::new(HashMap::from([(
            UiState::Overworld,
            vec![WORLD_TO_TERMINAL, WORLD_TO_BOTH],
        )]));

        let policy = NavigationPolicy::build(
            &graph,
            [UiState::Terminal, UiState::Archive(ArchiveState::Main)].into_iter(),
        )
        .unwrap();

        assert_eq!(
            policy.step(UiState::Overworld).unwrap().transition.outcomes,
            WORLD_TO_TERMINAL.outcomes
        );
    }

    #[test]
    fn leaves_unreachable_states_uncovered() {
        let graph = NavigationGraph::new(HashMap::new());

        let policy = NavigationPolicy::build(&graph, [UiState::Terminal].into_iter()).unwrap();

        assert!(!policy.covers(UiState::Overworld));
    }

    #[test]
    fn rejects_an_empty_destination_iterator() {
        let graph = NavigationGraph::new(HashMap::new());

        let error = match NavigationPolicy::build(&graph, std::iter::empty()) {
            Ok(_) => panic!("空目的状态集合应返回错误"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "导航策略至少需要一个目的状态");
    }
}
