//! 每日好运组合。按北京时间凌晨 4 点换日，最终生效的牌型与好运组合一致时得分翻倍。

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use ts_rs::TS;

use super::model::{self, HANDS, HandKind};

/// 好运组合每天轮换的牌型顺序。
const LUCKY_HANDS: [HandKind; 7] = [
    HandKind::Pair,
    HandKind::TwoPair,
    HandKind::Triplet,
    HandKind::Straight,
    HandKind::FullHouse,
    HandKind::Flush,
    HandKind::Quad,
];

/// 第 0 天是北京时间 2026-10-29 04:00 到 2026-10-30 04:00。
const LUCKY_EPOCH: (i32, u32, u32) = (2026, 10, 29);

/// 北京时间相对 `UTC` 的偏移，用于把凌晨 4 点的换日界换算成 `UTC` 的整天。
const BEIJING_OFFSET_SECONDS: i64 = 8 * 3600;

/// 换日时刻在北京时间一天中的位置。
const DAY_START_SECONDS: i64 = 4 * 3600;

const SECONDS_PER_DAY: i64 = 24 * 3600;

/// 单种牌型的基础得分倍率，单位为十分之一。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct HandMultiplier {
    pub hand: HandKind,
    pub tenths: u32,
}

/// 今日好运组合，以及界面标注倍率所需的数据。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct DailyLucky {
    /// 今日好运组合，最终生效的牌型与它一致时得分翻倍。
    pub hand: HandKind,
    /// 各牌型的基础倍率，按倍率从高到低排列。
    pub multipliers: Vec<HandMultiplier>,
    /// 距离下一次换日的秒数。
    pub next_change_in_seconds: u32,
}

pub(crate) fn daily(now: DateTime<Utc>) -> DailyLucky {
    DailyLucky {
        hand: hand(now),
        multipliers: HANDS
            .into_iter()
            .map(|hand| HandMultiplier {
                hand,
                tenths: model::multiplier_tenths(Some(hand)),
            })
            .collect(),
        next_change_in_seconds: next_change_in_seconds(now),
    }
}

/// 指定时刻的好运组合。
pub(crate) fn hand(now: DateTime<Utc>) -> HandKind {
    let index = day_index(now).rem_euclid(LUCKY_HANDS.len() as i64);
    LUCKY_HANDS[index as usize]
}

/// 北京时间为 `UTC+8`，凌晨 4 点换日等价于把 `UTC` 时间推后 4 小时后取日期。
fn day_index(now: DateTime<Utc>) -> i64 {
    (shifted_day(now) - epoch()).num_days()
}

fn shifted_day(now: DateTime<Utc>) -> NaiveDate {
    let offset = Duration::seconds(BEIJING_OFFSET_SECONDS - DAY_START_SECONDS);
    (now.naive_utc() + offset).date()
}

fn epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(LUCKY_EPOCH.0, LUCKY_EPOCH.1, LUCKY_EPOCH.2)
        .expect("固定的好运组合起始日期")
}

fn next_change_in_seconds(now: DateTime<Utc>) -> u32 {
    let into_day =
        (now.timestamp() + BEIJING_OFFSET_SECONDS - DAY_START_SECONDS).rem_euclid(SECONDS_PER_DAY);
    (SECONDS_PER_DAY - into_day) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rfc3339: &str) -> DateTime<Utc> {
        rfc3339.parse().expect("测试用时间应当合法")
    }

    #[test]
    fn rotates_the_lucky_hand_at_beijing_4am() {
        for (time, expected) in [
            // 第 0 天的起点，北京时间 2026-10-29 04:00。
            ("2026-10-28T20:00:00Z", HandKind::Pair),
            ("2026-10-29T19:59:59Z", HandKind::Pair),
            // 第 1 天的起点，北京时间 2026-10-30 04:00。
            ("2026-10-29T20:00:00Z", HandKind::TwoPair),
            ("2026-10-30T20:00:00Z", HandKind::Triplet),
            ("2026-11-04T20:00:00Z", HandKind::Pair),
            // 换日界之前的时刻仍属于前一天。
            ("2026-10-30T19:59:59Z", HandKind::TwoPair),
        ] {
            assert_eq!(hand(at(time)), expected, "{time}");
        }
    }

    #[test]
    fn counts_seconds_until_the_next_rotation() {
        assert_eq!(next_change_in_seconds(at("2026-10-28T20:00:00Z")), 86_400);
        assert_eq!(next_change_in_seconds(at("2026-10-28T21:00:00Z")), 82_800);
        assert_eq!(next_change_in_seconds(at("2026-10-29T19:59:59Z")), 1);
    }

    #[test]
    fn publishes_multipliers_from_high_to_low() {
        let daily = daily(at("2026-10-28T20:00:00Z"));
        assert_eq!(daily.hand, HandKind::Pair);
        assert_eq!(
            daily
                .multipliers
                .iter()
                .map(|item| (item.hand, item.tenths))
                .collect::<Vec<_>>(),
            vec![
                (HandKind::StraightFlush, 80),
                (HandKind::Quad, 60),
                (HandKind::Flush, 40),
                (HandKind::FullHouse, 20),
                (HandKind::Straight, 18),
                (HandKind::Triplet, 16),
                (HandKind::TwoPair, 14),
                (HandKind::Pair, 12),
            ]
        );
    }
}
