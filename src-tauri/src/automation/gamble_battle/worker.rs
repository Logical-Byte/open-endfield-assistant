//! 基于页面观测推进抽牌和放弃确认，命中目标后保留游戏画面。

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use image::RgbaImage;
use serde::Serialize;
use tracing::{debug, info, warn};
use ts_rs::TS;

use super::{
    model::{self, Card, Conditions},
    recognition::{Recognizer, count_cards},
    reporting::report_match,
};
use crate::{
    app_paths::AppPaths,
    automation::{
        self, Clock, Event, EventSink, Input, Point720p, ScreenCapture, StopToken,
        TemplateMatching, TemplateTarget, runtime, session::Session, stats::counts::Capture,
    },
    platform,
    utils::region::ltrb,
    vision,
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
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
}

impl Worker {
    pub(crate) fn new(conditions: Conditions, ocr: Arc<Mutex<vision::ocr::OcrEngine>>) -> Self {
        Self { conditions, ocr }
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
            let mut session = Session::connect(&self.ocr, stop)?;
            session.check_stop()?;
            if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
                warn!("无法将游戏窗口置于前台：{error:#}");
            }
            let mut cx = Capture::new(&mut session);
            let result = execute(&mut cx, &recognizer, &self.conditions, events.as_ref());
            capture = Some(cx.finish());
            result
        })();
        runtime::WorkerExit { result, capture }
    }
}

fn execute<C>(
    cx: &mut C,
    recognizer: &Recognizer,
    conditions: &Conditions,
    events: &dyn EventSink,
) -> Result<(), automation::WorkerError>
where
    C: ScreenCapture + Input + TemplateMatching + Clock,
{
    let started = Instant::now();
    let mut round = 1;
    cx.move_mouse_to_safe_position()?;
    let frame = cx.screenshot()?;
    require_target(cx, &frame, &ACTIVITY, "请先打开百趣演武活动的抽牌页面")?;
    info!("开始百趣演武自动化");
    loop {
        events.publish(Event::GambleBattleProgress(Progress {
            round,
            elapsed_seconds: started.elapsed().as_secs_f64(),
        }));
        let mut count = wait_for_card_count(cx, None)?;
        while count < 5 {
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
        // 额外等待一段时间以让卡牌颜色稳定
        cx.sleep(Duration::from_millis(500));
        // 抽满后才匹配数字和花色，截图和判定始终来自同一帧。
        let (full_frame, hand) = read_hand(cx, recognizer)?;
        let reasons = model::evaluate(&hand, conditions);
        if !reasons.is_empty() {
            report_match(
                events,
                hand,
                reasons,
                full_frame,
                Progress {
                    round,
                    elapsed_seconds: started.elapsed().as_secs_f64(),
                },
            )?;
            info!("百趣演武第 {round} 轮命中停止条件，已保留当前铭牌");
            return Ok(());
        }
        let button = wait_for_target(cx, &ABANDON, "未找到放弃挑战按钮")?;
        cx.click(button.region.center().into())?;
        wait_for_target(cx, &CONFIRM_ABANDON, "未出现放弃挑战的二次确认，已停止操作")?;
        cx.click(Point720p { x: 755, y: 466 })?;
        wait_for_card_count(cx, Some(0))?;
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

fn read_hand<C>(
    cx: &mut C,
    recognizer: &Recognizer,
) -> Result<(RgbaImage, [Card; 5]), automation::WorkerError>
where
    C: ScreenCapture + Clock,
{
    for _ in 0..40 {
        let frame = cx.screenshot()?;
        if count_cards(&frame).ok() == Some(5) {
            match recognizer.read_cards(&frame) {
                Ok(cards) => {
                    if let Ok(hand) = cards.try_into() {
                        return Ok((frame, hand));
                    }
                }
                Err(error) => debug!(%error, "等待五张铭牌的数字和花色清晰"),
            }
        }
        cx.sleep(Duration::from_millis(200));
    }
    Err(failure("无法识别五张铭牌的花色和战力点，已停止操作"))
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
