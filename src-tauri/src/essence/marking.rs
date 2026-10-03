//! 根据判断和当前标记选择目标操作。窗口点击与回读确认由自动化任务执行。

use serde::{Deserialize, Serialize};

use super::{Decision, Essence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkAction {
    Lock,
    Abandon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkingPlan {
    Disabled,
    Skipped,
    AlreadySet(MarkAction),
    Apply(MarkAction),
}

/// 根据同一份基质的 `evaluate` 结果规划标记。已锁定的基质不会被解锁或标记弃用。
pub fn plan_marking(essence: &Essence, decision: Decision, enabled: bool) -> MarkingPlan {
    if !enabled {
        return MarkingPlan::Disabled;
    }
    match decision {
        Decision::Keep if essence.locked == Some(true) => MarkingPlan::AlreadySet(MarkAction::Lock),
        Decision::Keep => MarkingPlan::Apply(MarkAction::Lock),
        Decision::Discard if essence.locked == Some(true) => MarkingPlan::Skipped,
        Decision::Discard if essence.abandoned == Some(true) => {
            MarkingPlan::AlreadySet(MarkAction::Abandon)
        }
        Decision::Discard => MarkingPlan::Apply(MarkAction::Abandon),
        Decision::Skip | Decision::Review => MarkingPlan::Skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essence::{Rarity, ScanSettings};

    fn item() -> Essence {
        Essence {
            stats: [
                "gat_passive_attr_str",
                "gat_passive_attr_atk",
                "gst_passive_tactic",
            ]
            .map(|id| Some(id.into())),
            levels: [Some(1); 3],
            rarity: Rarity::Five,
            locked: Some(false),
            abandoned: Some(false),
        }
    }

    #[test]
    fn default_settings_do_not_apply_marks() {
        let settings = ScanSettings::default();
        for decision in [Decision::Keep, Decision::Discard] {
            assert_eq!(
                plan_marking(&item(), decision, settings.auto_mark),
                MarkingPlan::Disabled
            );
        }
    }

    #[test]
    fn plans_new_marks_without_toggling_existing_marks_off() {
        let unmarked = item();
        assert_eq!(
            plan_marking(&unmarked, Decision::Keep, true),
            MarkingPlan::Apply(MarkAction::Lock)
        );
        assert_eq!(
            plan_marking(&unmarked, Decision::Discard, true),
            MarkingPlan::Apply(MarkAction::Abandon)
        );

        let locked = Essence {
            locked: Some(true),
            ..item()
        };
        assert_eq!(
            plan_marking(&locked, Decision::Keep, true),
            MarkingPlan::AlreadySet(MarkAction::Lock)
        );
        assert_eq!(
            plan_marking(&locked, Decision::Discard, true),
            MarkingPlan::Skipped
        );

        let abandoned = Essence {
            abandoned: Some(true),
            ..item()
        };
        assert_eq!(
            plan_marking(&abandoned, Decision::Discard, true),
            MarkingPlan::AlreadySet(MarkAction::Abandon)
        );
        assert_eq!(
            plan_marking(&abandoned, Decision::Keep, true),
            MarkingPlan::Apply(MarkAction::Lock)
        );
    }

    #[test]
    fn skipped_and_incomplete_items_have_no_marking_action() {
        let non_five_star = Essence {
            rarity: Rarity::Four,
            ..item()
        };
        assert_eq!(
            plan_marking(&non_five_star, Decision::Skip, true),
            MarkingPlan::Skipped
        );
        let mut incomplete = item();
        incomplete.stats[0] = None;
        assert_eq!(
            plan_marking(&incomplete, Decision::Review, true),
            MarkingPlan::Skipped
        );
    }
}
