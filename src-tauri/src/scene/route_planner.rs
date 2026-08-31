use std::collections::{HashMap, VecDeque};

use anyhow::{Result, bail};

use super::{SceneId, 档案库SubSceneId};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Route {
    pub(super) source: SceneId,
    pub(super) steps: Vec<(SceneId, SceneId)>,
}

/// 只包含逻辑跳转关系的不可变路由规划器。
pub(super) struct RoutePlanner {
    graph: HashMap<SceneId, Vec<SceneId>>,
}

impl RoutePlanner {
    pub(super) fn new(graph: HashMap<SceneId, Vec<SceneId>>) -> Self {
        Self { graph }
    }

    /// 使用 BFS 寻找最短路径，返回每一步的 `(来源场景, 目标场景)`。
    pub(super) fn find_route(&self, from: SceneId, to: SceneId) -> Result<Route> {
        if from == to {
            return Ok(Route {
                source: from,
                steps: Vec::new(),
            });
        }

        let lookup_from = normalize_scene_id(from);
        let mut queue = VecDeque::from([lookup_from]);
        let mut visited = HashMap::from([(lookup_from, lookup_from)]);

        while let Some(current) = queue.pop_front() {
            if let Some(destinations) = self.graph.get(&current) {
                for &next in destinations {
                    if let std::collections::hash_map::Entry::Vacant(entry) = visited.entry(next) {
                        entry.insert(current);
                        if next == to || normalize_scene_id(next) == normalize_scene_id(to) {
                            return Ok(Route {
                                source: from,
                                steps: reconstruct_steps(lookup_from, next, &visited),
                            });
                        }
                        queue.push_back(next);
                    }
                }
            }
        }

        bail!("导航失败: 无法从 {:?} 到达 {:?}", from, to)
    }
}

/// 导航图仅注册一个档案库子界面；规划时将其他子界面映射到该节点。
fn normalize_scene_id(id: SceneId) -> SceneId {
    match id {
        SceneId::档案库子界面(_) => {
            SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体)
        }
        other => other,
    }
}

fn reconstruct_steps(
    from: SceneId,
    to: SceneId,
    visited: &HashMap<SceneId, SceneId>,
) -> Vec<(SceneId, SceneId)> {
    let mut steps = Vec::new();
    let mut current = to;
    while current != from {
        let previous = visited[&current];
        steps.push((previous, current));
        current = previous;
    }
    steps.reverse();
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_source_and_destination_produces_empty_route() {
        let planner = RoutePlanner::new(HashMap::new());

        let route = planner
            .find_route(SceneId::档案库主界面, SceneId::档案库主界面)
            .unwrap();

        assert_eq!(route.source, SceneId::档案库主界面);
        assert!(route.steps.is_empty());
    }

    #[test]
    fn finds_shortest_route() {
        let planner = RoutePlanner::new(HashMap::from([
            (
                SceneId::大世界,
                vec![SceneId::协议终端, SceneId::档案库主界面],
            ),
            (SceneId::协议终端, vec![SceneId::档案详情页面]),
            (SceneId::档案库主界面, vec![SceneId::档案详情页面]),
        ]));

        let route = planner
            .find_route(SceneId::大世界, SceneId::档案详情页面)
            .unwrap();

        assert_eq!(
            route.steps,
            vec![
                (SceneId::大世界, SceneId::协议终端),
                (SceneId::协议终端, SceneId::档案详情页面),
            ]
        );
    }

    #[test]
    fn normalizes_archive_subscenes_for_lookup() {
        let registered_subscene = SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体);
        let planner = RoutePlanner::new(HashMap::from([(
            registered_subscene,
            vec![SceneId::档案库主界面],
        )]));

        let route = planner
            .find_route(
                SceneId::档案库子界面(档案库SubSceneId::见闻辑录_电子档案),
                SceneId::档案库主界面,
            )
            .unwrap();

        assert_eq!(
            route.steps,
            vec![(registered_subscene, SceneId::档案库主界面)]
        );
    }

    #[test]
    fn rejects_unreachable_destination() {
        let planner =
            RoutePlanner::new(HashMap::from([(SceneId::大世界, vec![SceneId::协议终端])]));

        let error = planner
            .find_route(SceneId::大世界, SceneId::档案库主界面)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "导航失败: 无法从 大世界 到达 档案库主界面"
        );
    }
}
