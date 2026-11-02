//! 百趣演武的铭牌、牌型与停止条件。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) enum Suit {
    Bing,
    Tiao,
    Wan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct Card {
    pub suit: Suit,
    pub rank: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) enum HandKind {
    StraightFlush,
    Quad,
    Flush,
    FullHouse,
    Straight,
    Triplet,
    TwoPair,
    Pair,
}

/// 启用的停止条件做析取，`None` 表示关闭对应的可选条件。
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct Conditions {
    pub straight_flush: bool,
    pub quad: bool,
    pub flush: bool,
    pub full_house: bool,
    pub straight: bool,
    pub triplet: bool,
    pub two_pair: bool,
    pub pair: bool,
    /// 战力点之和的下限。
    pub sum: Option<u8>,
    /// 分数的下限。
    pub score: Option<u32>,
    /// 按牌槽顺序拼接战力点后需要包含的子串，例如 `325`。
    pub pattern: Option<String>,
}

/// 命中的停止条件，可以同时包含最终牌型、战力点之和、分数和战力点子串。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) enum StopReason {
    Hand { hand: HandKind },
    Sum { threshold: u8, total: u8 },
    Score { threshold: u32, score: u32 },
    Pattern { pattern: String },
}

/// 达成目标时保留的铭牌及识别截图。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct MatchedHand {
    pub round: u32,
    pub elapsed_seconds: f64,
    pub cards: Vec<Card>,
    pub hand: Option<HandKind>,
    pub score: u32,
    /// 生效牌型是否为当天的好运组合，是则得分已经翻倍。
    pub lucky: bool,
    pub reasons: Vec<StopReason>,
    pub image: String,
}

/// 使用最终牌型匹配启用条件，不把高优先级牌型拆成较低牌型。
///
/// `early_stop` 为 `true` 时命中第一个条件就返回，调用方只关心是否存在命中。
pub(crate) fn evaluate(
    cards: &[Card],
    conditions: &Conditions,
    lucky: HandKind,
    early_stop: bool,
) -> Vec<StopReason> {
    let mut reasons = Vec::new();
    let hand = classify(cards);
    if let Some(hand) = hand
        && conditions.enables(hand)
    {
        reasons.push(StopReason::Hand { hand });
        if early_stop {
            return reasons;
        }
    }
    let total = total_bp(cards);
    if let Some(threshold) = conditions.sum
        && total >= threshold
    {
        reasons.push(StopReason::Sum { threshold, total });
        if early_stop {
            return reasons;
        }
    }
    let score = score_of(total, hand, lucky);
    if let Some(threshold) = conditions.score
        && score >= threshold
    {
        reasons.push(StopReason::Score { threshold, score });
        if early_stop {
            return reasons;
        }
    }
    if let Some(pattern) = &conditions.pattern
        && contains(cards, pattern)
    {
        reasons.push(StopReason::Pattern {
            pattern: pattern.clone(),
        });
    }
    reasons
}

/// 当前铭牌的战力点之和。
pub(crate) fn total_bp(cards: &[Card]) -> u8 {
    cards.iter().map(|card| card.rank).sum()
}

/// 当前铭牌的得分，等于战力点之和对应的基础得分乘以生效牌型的倍率。
pub(crate) fn score(cards: &[Card], lucky: HandKind) -> u32 {
    score_of(total_bp(cards), classify(cards), lucky)
}

/// 每张铭牌可能的花色数量（饼、条、万）。
const CARD_SUITS: u32 = 3;
/// 每张铭牌可能的战力点数量（1 到 5）。
const CARD_RANKS: u32 = 5;
/// 单张铭牌可能的取值数量。
const CARD_VARIANTS: u32 = CARD_SUITS * CARD_RANKS;

/// 判断本轮是否仍能命中停止条件。
/// `allow_early_stop` 开启时检查当前及后续任意前缀，关闭时只检查补满五张的结果。
///
/// 未翻开的铭牌按任意战力点与花色枚举，因此返回 `false` 表示本轮已不可能命中，
/// 可以提前放弃。枚举不考虑铭牌库的剩余数量，宁可多抽几张也不误弃可能命中的一轮。
pub(crate) fn can_still_match(
    prefix: &[Card],
    conditions: &Conditions,
    lucky: HandKind,
    allow_early_stop: bool,
) -> bool {
    let revealed = prefix.len().min(5);
    if revealed == 0 {
        return true;
    }
    let mut cards = [Card {
        suit: Suit::Bing,
        rank: 1,
    }; 5];
    cards[..revealed].copy_from_slice(&prefix[..revealed]);
    can_match_from(&mut cards, revealed, conditions, lucky, allow_early_stop)
}

/// 深度优先枚举后续抽牌，判断当前或后续任意前缀是否能命中。
///
/// 前提：`1 <= revealed <= 5`，`cards[..revealed]` 是已确定的合法铭牌，
/// `conditions` 已通过启动校验。空前缀由 `can_still_match` 直接返回 `true`，
/// 不进入递归。整次搜索使用同一个 `lucky`，不模拟搜索期间的换日。
///
/// 允许早停时每层先检查当前前缀，否则只在五张时检查。五张已命中返回 `true`，
/// 五张仍未命中返回 `false`。不足五张则枚举下一张的全部 15 种取值，
/// 找到一个可行分支就返回 `true`，全部分支失败才表示无解。
///
/// `cards[revealed..]` 只是可覆盖的搜索缓冲区，不参与当前判定。
/// 子调用保持当前前缀不变，后缀在读取前必定被赋值，因此回溯时无需恢复。
/// 每层增加一张，最多再递归四层。不考虑铭牌库余量，可能保留实际上抽不到的分支。
fn can_match_from(
    cards: &mut [Card; 5],
    revealed: usize,
    conditions: &Conditions,
    lucky: HandKind,
    allow_early_stop: bool,
) -> bool {
    if (allow_early_stop || revealed == 5)
        && !evaluate(&cards[..revealed], conditions, lucky, true).is_empty()
    {
        return true;
    }
    if revealed == 5 {
        return false;
    }
    for index in 0..CARD_VARIANTS {
        cards[revealed] = enumerated_card(index);
        if can_match_from(cards, revealed + 1, conditions, lucky, allow_early_stop) {
            return true;
        }
    }
    false
}

/// 战力点之和 1 点到 20 点及以上的基础得分，20 点以上不再增长。
const BASE_SCORES: [u32; 20] = [
    1000, 3000, 6000, 10000, 15000, 20000, 25000, 30000, 35000, 40000, 50000, 60000, 70000, 80000,
    90000, 100000, 110000, 120000, 130000, 150000,
];

fn base_score(total: u8) -> u32 {
    BASE_SCORES[usize::from(total.clamp(1, 20)) - 1]
}

/// 全部牌型，按倍率从高到低排列。
pub(super) const HANDS: [HandKind; 8] = [
    HandKind::StraightFlush,
    HandKind::Quad,
    HandKind::Flush,
    HandKind::FullHouse,
    HandKind::Straight,
    HandKind::Triplet,
    HandKind::TwoPair,
    HandKind::Pair,
];

/// 牌型倍率，以十分之一为单位表示，避免浮点误差。未构成牌型时倍率为 1。
pub(super) fn multiplier_tenths(hand: Option<HandKind>) -> u32 {
    match hand {
        Some(HandKind::StraightFlush) => 80,
        Some(HandKind::Quad) => 60,
        Some(HandKind::Flush) => 40,
        Some(HandKind::FullHouse) => 20,
        Some(HandKind::Straight) => 18,
        Some(HandKind::Triplet) => 16,
        Some(HandKind::TwoPair) => 14,
        Some(HandKind::Pair) => 12,
        None => 10,
    }
}

fn score_of(total: u8, hand: Option<HandKind>, lucky: HandKind) -> u32 {
    let lucky_doubling = if hand == Some(lucky) { 2 } else { 1 };
    base_score(total) * multiplier_tenths(hand) * lucky_doubling / 10
}

/// 枚举补全铭牌时的编号，按花色分组后每组对应 1 到 5 点战力点。
fn enumerated_card(index: u32) -> Card {
    let suit = match index / CARD_RANKS {
        0 => Suit::Bing,
        1 => Suit::Tiao,
        _ => Suit::Wan,
    };
    Card {
        suit,
        rank: (index % CARD_RANKS) as u8 + 1,
    }
}

/// 按牌槽顺序拼接当前铭牌的战力点，判断是否包含给定子串。
fn contains(cards: &[Card], pattern: &str) -> bool {
    let digits: Vec<u8> = cards.iter().map(|card| b'0' + card.rank).collect();
    digits
        .windows(pattern.len())
        .any(|window| window == pattern.as_bytes())
}

fn valid_pattern(pattern: &str) -> bool {
    (1..=5).contains(&pattern.len()) && pattern.bytes().all(|byte| (b'1'..=b'5').contains(&byte))
}

impl Conditions {
    /// 启动前校验，避免没有停止条件的抽卡循环。
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self
            .sum
            .is_some_and(|threshold| !(1..=25).contains(&threshold))
        {
            return Err("战力点阈值必须在 1 到 25 之间");
        }
        if self.score.is_some_and(|threshold| threshold == 0) {
            return Err("分数阈值必须大于 0");
        }
        if self
            .pattern
            .as_deref()
            .is_some_and(|pattern| !valid_pattern(pattern))
        {
            return Err("战力点子串必须由 1 到 5 的数字组成，长度为 1 到 5");
        }
        let hand_enabled = self.straight_flush
            || self.quad
            || self.flush
            || self.full_house
            || self.straight
            || self.triplet
            || self.two_pair
            || self.pair;
        if !hand_enabled && self.sum.is_none() && self.score.is_none() && self.pattern.is_none() {
            return Err("请至少启用一个停止条件");
        }
        Ok(())
    }

    fn enables(&self, hand: HandKind) -> bool {
        match hand {
            HandKind::StraightFlush => self.straight_flush,
            HandKind::Quad => self.quad,
            HandKind::Flush => self.flush,
            HandKind::FullHouse => self.full_house,
            HandKind::Straight => self.straight,
            HandKind::Triplet => self.triplet,
            HandKind::TwoPair => self.two_pair,
            HandKind::Pair => self.pair,
        }
    }
}

/// 返回当前铭牌中优先级最高的牌型。
pub(crate) fn classify(cards: &[Card]) -> Option<HandKind> {
    let counts: [usize; 5] = std::array::from_fn(|i| {
        cards
            .iter()
            .filter(|card| usize::from(card.rank) == i + 1)
            .count()
    });
    let flush = [Suit::Bing, Suit::Tiao, Suit::Wan]
        .iter()
        .any(|&suit| cards.iter().filter(|card| card.suit == suit).count() >= 4);
    if counts.iter().all(|&count| count == 1) && cards.iter().all(|card| card.suit == cards[0].suit)
    {
        Some(HandKind::StraightFlush)
    } else if counts.iter().any(|&count| count >= 4) {
        Some(HandKind::Quad)
    } else if flush {
        Some(HandKind::Flush)
    } else if counts.contains(&3) && counts.contains(&2) {
        Some(HandKind::FullHouse)
    } else if counts[..4].iter().all(|&count| count > 0)
        || counts[1..].iter().all(|&count| count > 0)
    {
        // 游戏中的顺子只需连续四个战力点。
        Some(HandKind::Straight)
    } else if counts.iter().any(|&count| count >= 3) {
        Some(HandKind::Triplet)
    } else if counts.iter().filter(|&&count| count == 2).count() == 2 {
        Some(HandKind::TwoPair)
    } else if counts.iter().any(|&count| count >= 2) {
        Some(HandKind::Pair)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards<const N: usize>(ranks: [u8; N], suits: [Suit; N]) -> [Card; N] {
        std::array::from_fn(|i| Card {
            rank: ranks[i],
            suit: suits[i],
        })
    }

    fn prefix(ranks: &[u8]) -> Vec<Card> {
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        ranks
            .iter()
            .enumerate()
            .map(|(slot, &rank)| Card {
                rank,
                suit: mixed[slot],
            })
            .collect()
    }

    #[test]
    fn classifies_five_cards_by_the_game_rules() {
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        for (ranks, suits, expected) in [
            ([2, 2, 2, 2, 3], mixed, HandKind::Quad),
            ([2, 2, 2, 2, 2], mixed, HandKind::Quad),
            ([2, 2, 2, 2, 3], [Suit::Wan; 5], HandKind::Quad),
            ([1, 1, 1, 3, 3], [Suit::Tiao; 5], HandKind::Flush),
            ([1, 1, 1, 3, 3], mixed, HandKind::FullHouse),
            ([1, 2, 3, 4, 4], mixed, HandKind::Straight),
            ([2, 3, 4, 5, 2], mixed, HandKind::Straight),
            ([1, 2, 3, 4, 5], mixed, HandKind::Straight),
            ([3, 3, 3, 1, 5], mixed, HandKind::Triplet),
            ([1, 1, 5, 5, 3], mixed, HandKind::TwoPair),
            ([1, 2, 3, 5, 5], mixed, HandKind::Pair),
        ] {
            assert_eq!(
                classify(&cards(ranks, suits)),
                Some(expected),
                "{ranks:?} {suits:?}"
            );
        }
    }

    #[test]
    fn classifies_partial_hands_without_lowering_hand_requirements() {
        for (ranks, expected) in [
            (&[2][..], None),
            (&[2, 2][..], Some(HandKind::Pair)),
            (&[2, 2, 2][..], Some(HandKind::Triplet)),
            (&[2, 2, 3, 3][..], Some(HandKind::TwoPair)),
            (&[2, 2, 2, 2][..], Some(HandKind::Quad)),
            (&[2, 3, 4, 5][..], Some(HandKind::Straight)),
        ] {
            assert_eq!(classify(&prefix(ranks)), expected, "{ranks:?}");
        }
        assert_eq!(classify(&cards([1, 2, 3], [Suit::Bing; 3])), None);
        assert_eq!(
            classify(&cards([1, 2, 3, 4], [Suit::Bing; 4])),
            Some(HandKind::Flush)
        );
        assert_eq!(
            classify(&cards(
                [1, 2, 3, 4, 5],
                [Suit::Bing, Suit::Bing, Suit::Bing, Suit::Bing, Suit::Tiao]
            )),
            Some(HandKind::Flush)
        );
    }

    #[test]
    fn matches_only_the_final_hand_and_combines_enabled_conditions() {
        let hand = cards(
            [2, 2, 2, 3, 3],
            [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao],
        );
        let lucky = HandKind::Pair;
        let mut conditions = Conditions {
            two_pair: true,
            ..Conditions::default()
        };
        assert!(evaluate(&hand, &conditions, lucky, false).is_empty());
        conditions.sum = Some(12);
        assert_eq!(
            evaluate(&hand, &conditions, lucky, false),
            vec![StopReason::Sum {
                threshold: 12,
                total: 12
            }]
        );
        conditions.full_house = true;
        assert_eq!(
            evaluate(&hand, &conditions, lucky, false),
            vec![
                StopReason::Hand {
                    hand: HandKind::FullHouse
                },
                StopReason::Sum {
                    threshold: 12,
                    total: 12
                },
            ]
        );
        conditions.sum = Some(13);
        assert_eq!(
            evaluate(&hand, &conditions, lucky, false),
            vec![StopReason::Hand {
                hand: HandKind::FullHouse
            }]
        );
    }

    #[test]
    fn evaluates_conditions_on_the_current_prefix() {
        let hand = prefix(&[2, 2, 2]);
        let conditions = Conditions {
            pair: true,
            sum: Some(6),
            score: Some(64_000),
            pattern: Some("22".into()),
            ..Conditions::default()
        };
        // 三条不命中对子。6 点基础分 20000，三条倍率 1.6，好运组合再翻倍。
        assert_eq!(
            evaluate(&hand, &conditions, HandKind::Triplet, false),
            vec![
                StopReason::Sum {
                    threshold: 6,
                    total: 6
                },
                StopReason::Score {
                    threshold: 64_000,
                    score: 64_000
                },
                StopReason::Pattern {
                    pattern: "22".into()
                },
            ]
        );
        assert_eq!(score(&hand, HandKind::Triplet), 64_000);
        assert_eq!(
            evaluate(&hand[..2], &conditions, HandKind::Triplet, false),
            vec![
                StopReason::Hand {
                    hand: HandKind::Pair
                },
                StopReason::Pattern {
                    pattern: "22".into()
                },
            ]
        );
    }

    #[test]
    fn requires_an_enabled_condition_and_an_in_range_sum_threshold() {
        assert!(Conditions::default().validate().is_err());
        for threshold in [0, 26, 255] {
            let conditions = Conditions {
                pair: true,
                sum: Some(threshold),
                ..Conditions::default()
            };
            assert!(conditions.validate().is_err());
        }
        for threshold in [1, 25] {
            let conditions = Conditions {
                sum: Some(threshold),
                ..Conditions::default()
            };
            assert!(conditions.validate().is_ok());
        }
        assert!(
            Conditions {
                flush: true,
                ..Conditions::default()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn straight_flush_takes_priority_over_flush_and_straight() {
        let hand = cards([5, 2, 4, 1, 3], [Suit::Bing; 5]);
        assert_eq!(classify(&hand), Some(HandKind::StraightFlush));
    }

    #[test]
    fn scores_the_hand_by_total_bp_and_the_best_hand_multiplier() {
        // 好运组合取对子，表中除最后两行外都不会翻倍。
        let lucky = HandKind::Pair;
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        for (ranks, suits, expected) in [
            ([5, 5, 5, 5, 5], mixed, 900_000),
            ([2, 2, 2, 2, 3], mixed, 300_000),
            ([1, 2, 3, 4, 5], [Suit::Bing; 5], 720_000),
            ([1, 1, 1, 3, 3], [Suit::Tiao; 5], 140_000),
            ([1, 1, 1, 2, 2], mixed, 50_000),
            ([1, 1, 2, 3, 4], mixed, 90_000),
            ([1, 1, 1, 2, 3], mixed, 48_000),
            ([1, 1, 3, 3, 2], mixed, 56_000),
            ([1, 1, 3, 4, 5], mixed, 192_000),
        ] {
            assert_eq!(
                score(&cards(ranks, suits), lucky),
                expected,
                "{ranks:?} {suits:?}"
            );
        }
    }

    #[test]
    fn keeps_only_completions_that_can_still_hit_the_score_target() {
        // 900000 分是 25 点四条的上限，只有四条 5 和补 5 点的四条 4 能命中。
        let conditions = Conditions {
            score: Some(900_000),
            ..Conditions::default()
        };
        let lucky = HandKind::Triplet;
        for (ranks, expected) in [
            (vec![], true),
            (vec![1], true),
            (vec![1, 2], false),
            (vec![4, 4], true),
            (vec![4, 5], true),
            (vec![5, 1], true),
            (vec![1, 3, 5], false),
            (vec![5, 5], true),
            (vec![5, 5, 1], true),
            (vec![4, 4, 1], false),
            (vec![4, 4, 4], true),
            (vec![5, 5, 5], true),
            (vec![1, 1, 1, 1], false),
            (vec![5, 5, 1, 2], false),
            (vec![5, 5, 1, 5], true),
            (vec![4, 4, 4, 1], false),
            (vec![4, 4, 4, 5], true),
            (vec![4, 4, 4, 4], true),
            (vec![5, 5, 5, 5], true),
        ] {
            assert_eq!(
                can_still_match(&prefix(&ranks), &conditions, lucky, true),
                expected,
                "{ranks:?}"
            );
        }
    }

    #[test]
    fn accounts_for_the_lucky_hand_when_checking_remaining_draws() {
        // 好运组合为四条时得分翻倍，四条只需 14 点即可达到 900000 分。
        let conditions = Conditions {
            score: Some(900_000),
            ..Conditions::default()
        };
        for (ranks, expected) in [
            (vec![1, 2], false),
            (vec![2, 2], false),
            (vec![3, 3], true),
            (vec![1, 1, 1], false),
            (vec![3, 3, 3], true),
            (vec![3, 3, 3, 3], true),
        ] {
            assert_eq!(
                can_still_match(&prefix(&ranks), &conditions, HandKind::Quad, true),
                expected,
                "{ranks:?}"
            );
        }
    }

    #[test]
    fn checks_sum_and_pattern_conditions_with_remaining_draws() {
        let sum = Conditions {
            sum: Some(20),
            ..Conditions::default()
        };
        for (ranks, expected) in [
            (vec![1, 2], false),
            (vec![1, 4], true),
            (vec![5, 5, 5, 5], true),
        ] {
            assert_eq!(
                can_still_match(&prefix(&ranks), &sum, HandKind::Pair, true),
                expected,
                "{ranks:?}"
            );
        }
        let pattern = Conditions {
            pattern: Some("5555".into()),
            ..Conditions::default()
        };
        for (ranks, expected) in [
            (vec![5, 5, 1], false),
            (vec![1, 5, 5], true),
            (vec![5, 5, 1, 5], false),
        ] {
            assert_eq!(
                can_still_match(&prefix(&ranks), &pattern, HandKind::Pair, true),
                expected,
                "{ranks:?}"
            );
        }
    }

    #[test]
    fn early_stop_preserves_whether_any_condition_matches() {
        let condition_sets = [
            Conditions {
                score: Some(900_000),
                ..Conditions::default()
            },
            Conditions {
                quad: true,
                sum: Some(18),
                score: Some(300_000),
                pattern: Some("25".into()),
                ..Conditions::default()
            },
            Conditions {
                straight_flush: true,
                full_house: true,
                sum: Some(20),
                ..Conditions::default()
            },
        ];
        let all_same = [Suit::Bing; 5];
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        let ranks_at = |seed: u32| -> [u8; 5] {
            std::array::from_fn(|slot| (seed / 5_u32.pow(slot as u32)) as u8 % 5 + 1)
        };
        // 取全同花色与混合花色，覆盖同花顺、同花及其他牌型。
        for seed in 0..5_u32.pow(5) {
            let values = ranks_at(seed);
            for suits in [all_same, mixed] {
                let hand = cards(values, suits);
                for conditions in &condition_sets {
                    for lucky in [HandKind::Triplet, HandKind::Quad] {
                        let full = evaluate(&hand, conditions, lucky, false);
                        let stopped = evaluate(&hand, conditions, lucky, true);
                        assert_eq!(
                            full.is_empty(),
                            stopped.is_empty(),
                            "{values:?} {suits:?} {conditions:?} {lucky:?}"
                        );
                        assert!(
                            full.starts_with(&stopped),
                            "{values:?} {suits:?} {conditions:?} {lucky:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn doubles_the_score_only_when_the_best_hand_is_the_lucky_hand() {
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        // 好运组合是对子时，两对仍然只按两对的基础倍率计分。
        let hand = cards([2, 2, 4, 4, 5], mixed);
        assert_eq!(classify(&hand), Some(HandKind::TwoPair));
        assert_eq!(score(&hand, HandKind::Pair), 154_000);
        assert_eq!(score(&hand, HandKind::TwoPair), 308_000);
    }

    #[test]
    fn combines_score_and_pattern_conditions() {
        let hand = cards(
            [3, 2, 5, 1, 1],
            [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao],
        );
        let lucky = HandKind::Quad;
        let mut conditions = Conditions {
            pattern: Some("325".into()),
            ..Conditions::default()
        };
        assert_eq!(
            evaluate(&hand, &conditions, lucky, false),
            vec![StopReason::Pattern {
                pattern: "325".into()
            }]
        );
        conditions.pattern = Some("235".into());
        assert!(evaluate(&hand, &conditions, lucky, false).is_empty());
        conditions.score = Some(72_000);
        assert_eq!(
            evaluate(&hand, &conditions, lucky, false),
            vec![StopReason::Score {
                threshold: 72_000,
                score: 72_000
            }]
        );
        conditions.score = Some(72_001);
        assert!(evaluate(&hand, &conditions, lucky, false).is_empty());
    }

    #[test]
    fn rejects_out_of_range_score_and_malformed_pattern() {
        for pattern in ["", "0", "6", "123456", "12a", "12 3"] {
            let conditions = Conditions {
                pattern: Some(pattern.into()),
                ..Conditions::default()
            };
            assert!(conditions.validate().is_err(), "{pattern:?}");
        }
        for pattern in ["1", "325", "12345"] {
            let conditions = Conditions {
                pattern: Some(pattern.into()),
                ..Conditions::default()
            };
            assert!(conditions.validate().is_ok(), "{pattern:?}");
        }
        let conditions = Conditions {
            score: Some(0),
            ..Conditions::default()
        };
        assert!(conditions.validate().is_err());
        let conditions = Conditions {
            score: Some(u32::MAX),
            ..Conditions::default()
        };
        assert!(conditions.validate().is_ok());
    }
}
