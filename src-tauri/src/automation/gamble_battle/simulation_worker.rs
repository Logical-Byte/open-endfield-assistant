//! 无需游戏窗口的百趣演武模拟，使用真实停止条件筛选样例牌组。

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use image::{Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_circle_mut},
    rect::Rect,
};

use super::{
    Progress, lucky,
    model::{self, Card, Conditions, Suit},
    reporting::report_match,
};
use crate::automation::{
    self, Event, EventSink, StopToken, capabilities, is_stop_requested, runtime,
};

pub(crate) struct SimulationWorker {
    conditions: Conditions,
}

impl SimulationWorker {
    pub(crate) fn new(conditions: Conditions) -> Self {
        Self { conditions }
    }
}

impl runtime::Worker for SimulationWorker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> runtime::WorkerExit {
        let result = simulate(&self.conditions, &stop, events.as_ref());
        runtime::WorkerExit {
            result,
            capture: None,
        }
    }
}

fn simulate(
    conditions: &Conditions,
    stop: &StopToken,
    events: &dyn EventSink,
) -> Result<(), automation::WorkerError> {
    conditions
        .validate()
        .map_err(|message| automation::WorkerError::Custom {
            message: message.into(),
        })?;
    let started = Instant::now();
    // 依次覆盖八种最终牌型和最高点数和，稀有条件也能在几秒内预览。
    // 是否命中完全交给真实判定，不能把样例标签当作成功原因。
    let samples = sample_hands();
    let mut round = 0;
    loop {
        let cards = samples[round as usize % samples.len()];
        round += 1;
        events.publish(Event::GambleBattleProgress(Progress {
            round,
            elapsed_seconds: started.elapsed().as_secs_f64(),
        }));
        let deadline = Instant::now() + Duration::from_millis(600);
        loop {
            if is_stop_requested(stop) {
                return Err(capabilities::Error::StoppedByUser.into());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            thread::sleep(remaining.min(Duration::from_millis(50)));
        }
        let lucky = lucky::hand(Utc::now());
        let reasons = model::evaluate(&cards, conditions, lucky);
        if !reasons.is_empty() {
            report_match(
                events,
                cards,
                reasons,
                lucky,
                sample_image(&cards),
                Progress {
                    round,
                    elapsed_seconds: started.elapsed().as_secs_f64(),
                },
            )?;
            return Ok(());
        }
    }
}

fn sample_hands() -> [[Card; 5]; 9] {
    let mixed = [Suit::Bing, Suit::Tiao, Suit::Wan, Suit::Bing, Suit::Tiao];
    [
        ([1, 2, 3, 5, 5], mixed),
        ([1, 1, 5, 5, 3], mixed),
        ([3, 3, 3, 1, 5], mixed),
        ([1, 2, 3, 4, 4], mixed),
        ([2, 2, 2, 3, 3], mixed),
        ([1, 1, 1, 3, 3], [Suit::Tiao; 5]),
        ([2, 2, 2, 2, 3], mixed),
        ([1, 2, 3, 4, 5], [Suit::Bing; 5]),
        ([5, 5, 5, 5, 5], mixed),
    ]
    .map(|(ranks, suits)| {
        std::array::from_fn(|i| Card {
            rank: ranks[i],
            suit: suits[i],
        })
    })
}

/// 生成与本轮牌组一致的示意截图。数字为点数，圆圈、竖条和 W 分别表示饼、条、万。
fn sample_image(cards: &[Card; 5]) -> RgbaImage {
    const DIGITS: [[u8; 5]; 5] = [
        [2, 6, 2, 2, 7],
        [7, 1, 7, 4, 7],
        [7, 1, 7, 1, 7],
        [5, 5, 7, 1, 1],
        [7, 4, 7, 1, 7],
    ];
    let ink = Rgba([42, 48, 58, 255]);
    let accent = Rgba([54, 156, 172, 255]);
    let mut image = RgbaImage::from_pixel(1280, 720, Rgba([30, 34, 42, 255]));
    for (slot, card) in cards.iter().enumerate() {
        let x = 105 + slot as i32 * 184;
        draw_filled_rect_mut(
            &mut image,
            Rect::at(x, 160).of_size(170, 270),
            Rgba([235, 238, 240, 255]),
        );
        for (y, bits) in DIGITS[usize::from(card.rank - 1)].iter().enumerate() {
            for column in 0..3 {
                if bits & (1 << (2 - column)) != 0 {
                    draw_filled_rect_mut(
                        &mut image,
                        Rect::at(x + 20 + column * 10, 185 + y as i32 * 10).of_size(10, 10),
                        ink,
                    );
                }
            }
        }
        match card.suit {
            Suit::Bing => {
                for radius in 20..27 {
                    draw_hollow_circle_mut(&mut image, (x + 85, 320), radius, accent);
                }
            }
            Suit::Tiao => {
                for offset in [0, 22, 44] {
                    draw_filled_rect_mut(
                        &mut image,
                        Rect::at(x + 58 + offset, 290).of_size(10, 60),
                        accent,
                    );
                }
            }
            Suit::Wan => {
                for (row, bits) in [17, 17, 21, 21, 10].iter().enumerate() {
                    for column in 0..5 {
                        if bits & (1 << (4 - column)) != 0 {
                            draw_filled_rect_mut(
                                &mut image,
                                Rect::at(x + 55 + column * 12, 290 + row as i32 * 12)
                                    .of_size(12, 12),
                                ink,
                            );
                        }
                    }
                }
            }
        }
    }
    image
}
