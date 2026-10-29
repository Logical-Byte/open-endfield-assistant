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
    FourOfAKind,
    Flush,
    FullHouse,
    Straight,
    ThreeOfAKind,
    TwoPairs,
    Pair,
}

/// 启用的停止条件做析取，`sum: None` 表示关闭战力点之和条件。
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct Conditions {
    pub straight_flush: bool,
    pub four_of_a_kind: bool,
    pub flush: bool,
    pub full_house: bool,
    pub straight: bool,
    pub three_of_a_kind: bool,
    pub two_pairs: bool,
    pub pair: bool,
    pub sum: Option<u8>,
}

/// 命中的停止条件，可以同时包含最终牌型和战力点之和。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) enum StopReason {
    Hand { hand: HandKind },
    Sum { threshold: u8, total: u8 },
}

/// 达成目标时保留的五张铭牌及识别截图。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct MatchedHand {
    pub round: u32,
    pub elapsed_seconds: f64,
    pub cards: [Card; 5],
    pub hand: Option<HandKind>,
    pub reasons: Vec<StopReason>,
    pub image: String,
}

/// 使用最终牌型匹配启用条件，不把高优先级牌型拆成较低牌型。
pub(crate) fn evaluate(cards: &[Card; 5], conditions: &Conditions) -> Vec<StopReason> {
    let mut reasons = Vec::new();
    if let Some(hand) = classify(cards)
        && conditions.enables(hand)
    {
        reasons.push(StopReason::Hand { hand });
    }
    let total = cards.iter().map(|card| card.rank).sum();
    if let Some(threshold) = conditions.sum
        && total >= threshold
    {
        reasons.push(StopReason::Sum { threshold, total });
    }
    reasons
}

impl Conditions {
    /// 启动前校验，避免没有停止条件的抽卡循环。
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self
            .sum
            .is_some_and(|threshold| !(1..=25).contains(&threshold))
        {
            return Err("战力点之和阈值必须在 1 到 25 之间");
        }
        let hand_enabled = self.straight_flush
            || self.four_of_a_kind
            || self.flush
            || self.full_house
            || self.straight
            || self.three_of_a_kind
            || self.two_pairs
            || self.pair;
        if !hand_enabled && self.sum.is_none() {
            return Err("请至少启用一个停止条件");
        }
        Ok(())
    }

    fn enables(&self, hand: HandKind) -> bool {
        match hand {
            HandKind::StraightFlush => self.straight_flush,
            HandKind::FourOfAKind => self.four_of_a_kind,
            HandKind::Flush => self.flush,
            HandKind::FullHouse => self.full_house,
            HandKind::Straight => self.straight,
            HandKind::ThreeOfAKind => self.three_of_a_kind,
            HandKind::TwoPairs => self.two_pairs,
            HandKind::Pair => self.pair,
        }
    }
}

/// 返回五张铭牌中优先级最高的牌型。
pub(crate) fn classify(cards: &[Card; 5]) -> Option<HandKind> {
    let counts: [usize; 5] = std::array::from_fn(|i| {
        cards
            .iter()
            .filter(|card| usize::from(card.rank) == i + 1)
            .count()
    });
    let flush = cards.iter().all(|card| card.suit == cards[0].suit);
    if flush && counts.iter().all(|&count| count == 1) {
        Some(HandKind::StraightFlush)
    } else if counts.iter().any(|&count| count >= 4) {
        Some(HandKind::FourOfAKind)
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
        Some(HandKind::ThreeOfAKind)
    } else if counts.iter().filter(|&&count| count == 2).count() == 2 {
        Some(HandKind::TwoPairs)
    } else if counts.iter().any(|&count| count >= 2) {
        Some(HandKind::Pair)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(ranks: [u8; 5], suits: [Suit; 5]) -> [Card; 5] {
        std::array::from_fn(|i| Card {
            rank: ranks[i],
            suit: suits[i],
        })
    }

    #[test]
    fn classifies_five_cards_by_the_game_rules() {
        let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
        for (ranks, suits, expected) in [
            ([2, 2, 2, 2, 3], mixed, HandKind::FourOfAKind),
            ([2, 2, 2, 2, 2], mixed, HandKind::FourOfAKind),
            ([2, 2, 2, 2, 3], [Suit::Wan; 5], HandKind::FourOfAKind),
            ([1, 1, 1, 3, 3], [Suit::Tiao; 5], HandKind::Flush),
            ([1, 1, 1, 3, 3], mixed, HandKind::FullHouse),
            ([1, 2, 3, 4, 4], mixed, HandKind::Straight),
            ([2, 3, 4, 5, 2], mixed, HandKind::Straight),
            ([1, 2, 3, 4, 5], mixed, HandKind::Straight),
            ([3, 3, 3, 1, 5], mixed, HandKind::ThreeOfAKind),
            ([1, 1, 5, 5, 3], mixed, HandKind::TwoPairs),
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
    fn matches_only_the_final_hand_and_combines_enabled_conditions() {
        let hand = cards(
            [2, 2, 2, 3, 3],
            [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao],
        );
        let mut conditions = Conditions {
            two_pairs: true,
            ..Conditions::default()
        };
        assert!(evaluate(&hand, &conditions).is_empty());
        conditions.sum = Some(12);
        assert_eq!(
            evaluate(&hand, &conditions),
            vec![StopReason::Sum {
                threshold: 12,
                total: 12
            }]
        );
        conditions.full_house = true;
        assert_eq!(
            evaluate(&hand, &conditions),
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
            evaluate(&hand, &conditions),
            vec![StopReason::Hand {
                hand: HandKind::FullHouse
            }]
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
}
