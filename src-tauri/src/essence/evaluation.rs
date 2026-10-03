use serde::{Deserialize, Serialize};

use super::{Catalog, Essence, NonFiveStar, Rarity, ScanSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Keep,
    Discard,
    Skip,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    Locked,
    Abandoned,
    NonFiveStar,
    CustomRule,
    HighLevel,
    WeaponMatch,
    ExcludedWeapons,
    NoMatchingWeapon,
    IncompleteRecognition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    pub decision: Decision,
    pub reason: Reason,
    /// 全部匹配的武器，包括用户已排除的武器，供结果界面解释判断。
    pub matched_weapon_ids: Vec<String>,
}

impl Evaluation {
    fn without_matches(decision: Decision, reason: Reason) -> Self {
        Self {
            decision,
            reason,
            matched_weapon_ids: Vec::new(),
        }
    }
}

/// 以已保存的规则判断一份基质，不读取设置、不修改标记，也不依赖扫描顺序。
pub fn evaluate(essence: &Essence, settings: &ScanSettings, catalog: &Catalog) -> Evaluation {
    if settings.protect_locked && essence.locked == Some(true) {
        return Evaluation::without_matches(Decision::Keep, Reason::Locked);
    }
    if settings.skip_abandoned && essence.abandoned == Some(true) {
        return Evaluation::without_matches(Decision::Skip, Reason::Abandoned);
    }
    if settings.non_five_star == NonFiveStar::Skip
        && matches!(essence.rarity, Rarity::Four | Rarity::Other)
    {
        return Evaluation::without_matches(Decision::Skip, Reason::NonFiveStar);
    }
    if essence.rarity == Rarity::Unknown || essence.locked.is_none() || essence.abandoned.is_none()
    {
        return Evaluation::without_matches(Decision::Review, Reason::IncompleteRecognition);
    }

    let mut stats_by_kind = [None; 3];
    let mut high_level = false;
    for (id, level) in essence.stats.iter().zip(essence.levels) {
        let Some((stat, level)) = id
            .as_deref()
            .and_then(|id| catalog.stat(id))
            .zip(level)
            .filter(|(stat, level)| (1..=stat.kind.max_level()).contains(level))
        else {
            return Evaluation::without_matches(Decision::Review, Reason::IncompleteRecognition);
        };
        let index = stat.kind.index();
        // 同类型可能出现多个词条。与 EER 一致，武器组合取该类型首次出现的属性。
        stats_by_kind[index].get_or_insert(stat.id.as_str());
        if let Some(thresholds) = settings.high_level {
            high_level |= level >= thresholds[index];
        }
    }

    let matched_weapon_ids: Vec<String> = catalog
        .weapons
        .iter()
        .filter(|weapon| weapon.stats.each_ref().map(|id| id.as_deref()) == stats_by_kind)
        .map(|weapon| weapon.id.clone())
        .collect();

    let (decision, reason) = if settings
        .custom_keeps
        .iter()
        .any(|rule| rule.each_ref().map(|id| Some(id.as_str())) == stats_by_kind)
    {
        (Decision::Keep, Reason::CustomRule)
    } else if high_level {
        (Decision::Keep, Reason::HighLevel)
    } else if matched_weapon_ids
        .iter()
        .any(|id| !settings.excluded_weapon_ids.contains(id))
    {
        (Decision::Keep, Reason::WeaponMatch)
    } else if matched_weapon_ids.is_empty() {
        (Decision::Discard, Reason::NoMatchingWeapon)
    } else {
        (Decision::Discard, Reason::ExcludedWeapons)
    };

    Evaluation {
        decision,
        reason,
        matched_weapon_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essence::{Stat, StatKind, Weapon};

    fn catalog() -> Catalog {
        Catalog {
            stats: [
                ("strength", StatKind::Attribute),
                ("agility", StatKind::Attribute),
                ("attack", StatKind::Secondary),
                ("skill", StatKind::Skill),
            ]
            .into_iter()
            .map(|(id, kind)| Stat {
                id: id.into(),
                name: id.into(),
                kind,
            })
            .collect(),
            weapons: ["sword", "spear"]
                .into_iter()
                .map(|id| Weapon {
                    id: id.into(),
                    name: id.into(),
                    rarity: 6,
                    stats: ["strength", "attack", "skill"].map(|id| Some(id.into())),
                })
                .collect(),
        }
    }

    fn essence() -> Essence {
        Essence {
            stats: ["strength", "attack", "skill"].map(|id| Some(id.into())),
            levels: [Some(1); 3],
            rarity: Rarity::Five,
            locked: Some(false),
            abandoned: Some(false),
        }
    }

    #[test]
    fn incomplete_recognition_never_becomes_discard() {
        let catalog = catalog();
        let settings = ScanSettings::default();
        let mut variants = vec![essence(); 7];
        variants[0].rarity = Rarity::Unknown;
        variants[1].stats[0] = None;
        variants[2].levels[0] = None;
        variants[3].locked = None;
        variants[4].abandoned = None;
        variants[5].stats[0] = Some("unknown-stat".into());
        variants[6].levels[2] = Some(6);
        for variant in variants {
            assert_eq!(
                evaluate(&variant, &settings, &catalog),
                Evaluation::without_matches(Decision::Review, Reason::IncompleteRecognition)
            );
        }
    }

    #[test]
    fn matches_weapons_by_semantic_kind_instead_of_screen_row() {
        let mut essence = essence();
        essence.stats.swap(0, 2);
        let evaluation = evaluate(&essence, &ScanSettings::default(), &catalog());
        assert_eq!(evaluation.decision, Decision::Keep);
        assert_eq!(evaluation.reason, Reason::WeaponMatch);
        assert_eq!(evaluation.matched_weapon_ids, ["sword", "spear"]);
    }

    #[test]
    fn high_level_uses_semantic_thresholds_even_when_all_weapons_are_excluded() {
        let mut essence = essence();
        essence.stats.swap(0, 2);
        essence.levels = [Some(3), Some(1), Some(1)];
        let mut settings = ScanSettings {
            excluded_weapon_ids: vec!["sword".into(), "spear".into()],
            ..ScanSettings::default()
        };
        assert_eq!(
            evaluate(&essence, &settings, &catalog()).reason,
            Reason::ExcludedWeapons
        );
        settings.high_level = Some([6, 6, 3]);
        let evaluation = evaluate(&essence, &settings, &catalog());
        assert_eq!(evaluation.decision, Decision::Keep);
        assert_eq!(evaluation.reason, Reason::HighLevel);
        assert_eq!(evaluation.matched_weapon_ids, ["sword", "spear"]);
    }

    #[test]
    fn custom_keep_requires_the_complete_semantic_tuple() {
        let mut essence = essence();
        essence.stats[0] = Some("agility".into());
        essence.stats.swap(0, 1);
        let mut settings = ScanSettings {
            custom_keeps: vec![["agility", "attack", "skill"].map(String::from)],
            ..ScanSettings::default()
        };
        let evaluation = evaluate(&essence, &settings, &catalog());
        assert_eq!(evaluation.decision, Decision::Keep);
        assert_eq!(evaluation.reason, Reason::CustomRule);
        assert!(evaluation.matched_weapon_ids.is_empty());
        settings.custom_keeps[0][0] = "strength".into();
        assert_eq!(
            evaluate(&essence, &settings, &catalog()).reason,
            Reason::NoMatchingWeapon
        );
    }

    #[test]
    fn explicit_marker_protection_and_skips_precede_matching() {
        let mut essence = essence();
        essence.rarity = Rarity::Four;
        essence.locked = Some(true);
        essence.abandoned = Some(true);
        essence.stats[0] = None;
        let mut settings = ScanSettings {
            non_five_star: NonFiveStar::Skip,
            skip_abandoned: true,
            ..ScanSettings::default()
        };
        assert_eq!(
            evaluate(&essence, &settings, &catalog()),
            Evaluation::without_matches(Decision::Keep, Reason::Locked)
        );
        settings.protect_locked = false;
        assert_eq!(
            evaluate(&essence, &settings, &catalog()),
            Evaluation::without_matches(Decision::Skip, Reason::Abandoned)
        );
        settings.skip_abandoned = false;
        assert_eq!(
            evaluate(&essence, &settings, &catalog()),
            Evaluation::without_matches(Decision::Skip, Reason::NonFiveStar)
        );
    }

    #[test]
    fn rejects_rules_that_cannot_be_applied_to_the_catalog() {
        let mut settings = ScanSettings {
            high_level: Some([6, 6, 4]),
            ..ScanSettings::default()
        };
        assert!(settings.validate(&catalog()).is_err());
        settings.high_level = Some([6, 6, 3]);
        settings.custom_keeps = vec![["attack", "strength", "skill"].map(String::from)];
        assert!(settings.validate(&catalog()).is_err());
        settings.custom_keeps = vec![["strength", "attack", "skill"].map(String::from)];
        assert!(settings.validate(&catalog()).is_ok());
    }
}
