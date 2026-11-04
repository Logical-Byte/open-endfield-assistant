//! 基于页面观测推进抽牌和放弃确认，命中目标后保留游戏画面。

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use image::RgbaImage;
use oea_vision as vision;
use serde::Serialize;
use tracing::{debug, info, warn};
use ts_rs::TS;

use super::{
    lucky,
    model::{self, Card, Conditions},
    recognition::{Recognizer, count_cards},
    reporting::report_match,
};
use crate::{
    app_paths::AppPaths,
    automation::{
        self, Clock, Event, EventSink, Input, Point720p, ScreenCapture, StopToken,
        TemplateMatching, TemplateTarget, runtime,
        session::Session,
        sound::{self, Cue},
        stats::counts::Capture,
        template_loader::TemplateLoader,
    },
    utils::region::ltrb,
};

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "gambleBattle/")]
pub(crate) struct Progress {
    pub round: u32,
    pub elapsed_seconds: f64,
}

pub(crate) struct Worker {
    conditions: Conditions,
    allow_early_stop: bool,
    sound_volume: f32,
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
}

impl Worker {
    pub(crate) fn new(
        conditions: Conditions,
        allow_early_stop: bool,
        sound_volume: f32,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    ) -> Self {
        Self {
            conditions,
            allow_early_stop,
            sound_volume,
            ocr,
        }
    }
}

impl runtime::Worker for Worker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> runtime::WorkerExit {
        let mut capture = None;
        let result = (|| {
            self.conditions.validate().map_err(failure)?;
            let paths = AppPaths::new().map_err(|error| failure(error.to_string()))?;
            let recognizer = Recognizer::new(&paths.templates_dir())
                .map_err(|error| failure(format!("加载百趣演武模板失败：{error}")))?;
            let mut session =
                Session::connect(&self.ocr, TemplateLoader::new(paths.templates_dir()), stop)?;
            session.check_stop()?;
            if let Err(error) = session.window.activate() {
                warn!("无法将游戏窗口置于前台：{error:#}");
            }
            sound::play(self.sound_volume, Cue::Enable);
            let mut cx = Capture::new(&mut session);
            let result = execute(
                &mut cx,
                &recognizer,
                &self.conditions,
                self.allow_early_stop,
                events.as_ref(),
            );
            capture = Some(cx.finish());
            result
        })();
        let cue = if result.is_ok() {
            Cue::Enable
        } else {
            Cue::Disable
        };
        sound::play(self.sound_volume, cue);
        runtime::WorkerExit { result, capture }
    }
}

fn execute<C>(
    cx: &mut C,
    recognizer: &Recognizer,
    conditions: &Conditions,
    allow_early_stop: bool,
    events: &dyn EventSink,
) -> Result<(), automation::WorkerError>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    let started = Instant::now();
    let mut round = 1;
    cx.move_mouse_to_safe_position()?;
    let frame = cx.screenshot()?;
    require_target(cx, &frame, &ACTIVITY, "请先打开百趣演武活动的抽牌界面")?;
    info!("开始百趣演武自动化");
    loop {
        events.publish(Event::GambleBattleProgress(Progress {
            round,
            elapsed_seconds: started.elapsed().as_secs_f64(),
        }));
        let mut count = wait_for_card_count(cx, None)?;
        // 每个牌数只检查一次。识别不稳定时继续运行，避免反复卡在同一组铭牌上。
        let mut checked_count = None;
        loop {
            let mut impossible = false;
            if count > 0 && checked_count != Some(count) {
                checked_count = Some(count);
                if count == 5 {
                    // 抽满后额外等待卡牌颜色稳定。
                    cx.sleep(Duration::from_millis(500));
                }
                if let Some((frame, cards)) = read_hand(cx, recognizer, count)? {
                    require_target(cx, &frame, &ACTIVITY, "百趣演武页面已离开，停止抽牌")?;
                    // 每次判定重新取好运组合，跨过凌晨 4 点时按新的一天计分。
                    let lucky = lucky::hand(Utc::now());
                    let reasons = if allow_early_stop || count == 5 {
                        model::evaluate(&cards, conditions, lucky, false)
                    } else {
                        Vec::new()
                    };
                    if !reasons.is_empty() {
                        report_match(
                            events,
                            &cards,
                            reasons,
                            lucky,
                            frame,
                            Progress {
                                round,
                                elapsed_seconds: started.elapsed().as_secs_f64(),
                            },
                        )?;
                        info!("百趣演武第 {round} 轮命中停止条件，已保留当前 {count} 张铭牌");
                        return Ok(());
                    }
                    impossible =
                        !model::can_still_match(&cards, conditions, lucky, allow_early_stop);
                }
            }
            if count == 5 || impossible {
                if impossible && count < 5 {
                    info!(
                        "百趣演武第 {round} 轮翻出 {count} 张铭牌后已不可能命中停止条件，提前放弃"
                    );
                }
                abandon_round(cx)?;
                break;
            }
            let frame = cx.screenshot()?;
            require_target(cx, &frame, &ACTIVITY, "百趣演武页面已离开，停止抽牌")?;
            // 抽牌按钮会闪烁，等待可识别的帧后再点击。`Session::click` 会将光标归中。
            let button = wait_for_target(cx, &DRAW, "未找到可用的抽牌按钮")?;
            cx.click(button.region.center().into())?;
            // 动画期间按钮可能忽略点击，后续按实际牌数决定是否继续抽牌。
            cx.sleep(Duration::from_millis(500));
            count = wait_for_card_count(cx, None)?;
            events.publish(Event::GambleBattleProgress(Progress {
                round,
                elapsed_seconds: started.elapsed().as_secs_f64(),
            }));
        }
        round += 1;
    }
}

/// 动画中允许暂时无法识别。只有两次连续一致的观测才用于推进操作。
fn wait_for_card_count<C>(
    cx: &mut C,
    expected: Option<usize>,
) -> Result<usize, automation::WorkerError>
where
    C: ScreenCapture + Clock,
{
    let mut previous = None;
    for _ in 0..40 {
        let frame = cx.screenshot()?;
        match count_cards(&frame) {
            Ok(count) if expected.is_none_or(|expected| count == expected) => {
                if previous == Some(count) {
                    return Ok(count);
                }
                previous = Some(count);
            }
            result => {
                debug!(?result, ?expected, "等待铭牌动画结束");
                previous = None;
            }
        }
        cx.sleep(Duration::from_millis(200));
    }
    Err(failure(
        "无法确认铭牌状态，已停止操作。请检查活动页面或识别模板",
    ))
}

/// 连续两次识别一致后才用于判定，返回的截图与铭牌来自同一帧。
/// 无法稳定识别时继续运行，满五张则由调用方清空并开始下一轮。
fn read_hand<C>(
    cx: &mut C,
    recognizer: &Recognizer,
    count: usize,
) -> Result<Option<(RgbaImage, Vec<Card>)>, automation::WorkerError>
where
    C: ScreenCapture + Clock,
{
    let mut previous: Option<Vec<Card>> = None;
    for _ in 0..5 {
        let frame = cx.screenshot()?;
        if count_cards(&frame).ok() == Some(count)
            && let Ok(cards) = recognizer.read_cards(&frame)
            && cards.len() == count
        {
            if previous.as_deref() == Some(cards.as_slice()) {
                return Ok(Some((frame, cards)));
            }
            previous = Some(cards);
        } else {
            previous = None;
        }
        cx.sleep(Duration::from_millis(150));
    }
    debug!("未能稳定识别 {count} 张已翻开的铭牌，继续运行");
    Ok(None)
}

/// 放弃当前一轮，等待回到没有铭牌的抽牌界面。
fn abandon_round<C>(cx: &mut C) -> Result<(), automation::WorkerError>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    let button = wait_for_target(cx, &ABANDON, "未找到放弃挑战按钮")?;
    cx.click(button.region.center().into())?;
    wait_for_target(cx, &CONFIRM_ABANDON, "未出现放弃挑战的二次确认，已停止操作")?;
    cx.click(Point720p { x: 755, y: 466 })?;
    wait_for_card_count(cx, Some(0))?;
    Ok(())
}

fn require_target<C: TemplateMatching>(
    cx: &mut C,
    frame: &RgbaImage,
    target: &TemplateTarget,
    message: &str,
) -> Result<automation::TemplateMatch, automation::WorkerError> {
    cx.find_template(frame, target)?
        .ok_or_else(|| failure(message))
}

fn wait_for_target<C>(
    cx: &mut C,
    target: &TemplateTarget,
    message: &str,
) -> Result<automation::TemplateMatch, automation::WorkerError>
where
    C: ScreenCapture + TemplateMatching + Clock,
{
    for _ in 0..40 {
        let frame = cx.screenshot()?;
        if let Some(matched) = cx.find_template(&frame, target)? {
            return Ok(matched);
        }
        cx.sleep(Duration::from_millis(200));
    }
    Err(failure(message))
}

fn failure(message: impl Into<String>) -> automation::WorkerError {
    automation::WorkerError::Custom {
        message: message.into(),
    }
}

const ACTIVITY: TemplateTarget = TemplateTarget {
    template_name: "百趣演武/activity.png",
    roi: ltrb!(1125, 100, 1265, 140),
    threshold: 0.85,
};
const DRAW: TemplateTarget = TemplateTarget {
    template_name: "百趣演武/draw.png",
    roi: ltrb!(1180, 520, 1265, 575),
    threshold: 0.85,
};
const ABANDON: TemplateTarget = TemplateTarget {
    template_name: "百趣演武/abandon.png",
    roi: ltrb!(830, 630, 958, 686),
    threshold: 0.85,
};
const CONFIRM_ABANDON: TemplateTarget = TemplateTarget {
    template_name: "百趣演武/confirm-abandon.png",
    roi: ltrb!(420, 330, 860, 390),
    threshold: 0.85,
};
