//! 真实档案扫描的工作线程和完整执行流程。

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use image::RgbaImage;
use tracing::{debug, info, warn};

use crate::{
    app_paths::AppPaths,
    automation::{
        AutomationStopped, Clock, EventSink, Input, Ocr, ScreenCapture, StopToken,
        TemplateMatching, TemplateTarget,
        runtime::{self, FinishReason, WorkerExit},
        session::Session,
        stats::counts::{Capture, CaptureSummary},
    },
    data::{AppData, archive},
    navigation::{ArchiveState, ArchiveSubscene, CentralPage, Navigator, RecordsPage, UiState},
    platform, settings,
    utils::region::{Region2D, ltrb, ltwh},
    vision,
};

use super::{ocr_correction, reporting::ScanReporter};

/// 一次真实扫描所需的协作者，由控制器在任务获准启动后创建。
pub(crate) struct Worker {
    settings: settings::OeaSettings,
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    navigator: Arc<Navigator>,
    app_data: Arc<AppData>,
}

impl Worker {
    pub(crate) fn new(
        settings: settings::OeaSettings,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
        navigator: Arc<Navigator>,
        app_data: Arc<AppData>,
    ) -> Self {
        Self {
            settings,
            ocr,
            navigator,
            app_data,
        }
    }
}

impl runtime::Worker for Worker {
    fn run(self: Box<Self>, stop: StopToken, events: Arc<dyn EventSink>) -> WorkerExit {
        let mut capture: Option<CaptureSummary> = None;
        let result = execute_session(&self, stop, events, &mut capture);
        let exit = WorkerExit {
            reason: match result {
                Ok(()) => FinishReason::Completed,
                Err(error) if error.downcast_ref::<AutomationStopped>().is_some() => {
                    FinishReason::Stopped
                }
                Err(error) => FinishReason::Failed(format!("{error:#}")),
            },
            capture,
        };
        let sound = match &exit.reason {
            FinishReason::Completed => ScanSound::Enable,
            FinishReason::Stopped | FinishReason::Failed(_) => ScanSound::Disable,
        };
        play_scan_sound(self.settings.sound_volume, sound);
        exit
    }
}

fn execute_session(
    worker: &Worker,
    stop: StopToken,
    events: Arc<dyn EventSink>,
    capture: &mut Option<CaptureSummary>,
) -> anyhow::Result<()> {
    // 连接游戏可能耗时，所以留在工作线程中。
    let mut session = Session::connect(&worker.ocr, stop).context("连接游戏失败")?;

    // 停止请求可能发生在连接过程中，不能让它被清除或跳过。
    session.check_stop()?;

    // 扫描需要点击游戏窗口，先确保窗口在前台（失败不阻断）。
    if let Err(error) = platform::window::ensure_foreground_and_topmost(session.hwnd) {
        warn!("无法将游戏窗口置于前台: {error:#}，继续尝试执行任务");
    }

    // 启动检查通过、任务真正开始执行前播放 enable 提示音。
    play_scan_sound(worker.settings.sound_volume, ScanSound::Enable);

    let reporter = ScanReporter::new(events);
    let mut captured_session = Capture::new(&mut session);
    let result = scan_archives(
        &mut captured_session,
        &worker.navigator,
        worker.app_data.archives(),
        &reporter,
    );
    // 扫描失败或停止时也收集统计，再向外传播结果。
    *capture = Some(captured_session.finish());
    result.context("扫描档案库任务执行失败")
}

/// 移动鼠标、进入档案库主界面，然后按顺序扫描全部子分类。
fn scan_archives<C>(
    cx: &mut C,
    navigator: &Navigator,
    archives: &archive::Database,
    reporter: &ScanReporter,
) -> anyhow::Result<()>
where
    C: ScreenCapture + Input + TemplateMatching + Ocr + Clock,
{
    info!("========== 开始执行任务: 扫描档案库 ==========");

    // 避免鼠标 hover 样式变化干扰首次 UI 状态识别和导航。
    cx.move_mouse_to_safe_position()?;
    navigator.navigate_to(UiState::Archive(ArchiveState::Main), cx)?;

    for (index, &subscene) in SCAN_PLAN.iter().enumerate() {
        info!(
            "===== 扫描子分类 {}/{}: {} =====",
            index + 1,
            SCAN_PLAN.len(),
            subscene
        );
        navigator.navigate_to(UiState::archive_subscene(subscene), cx)?;
        scan_subscene(cx, navigator, subscene, archives, reporter)?;
        info!("完成扫描 {subscene}");
    }

    info!("全部 6 个子分类扫描完毕！");
    info!("========== 任务 扫描档案库 执行完毕 ==========");
    Ok(())
}

/// 从当前子分类进入详情，逐份扫描，完成后返回该子分类。
fn scan_subscene<C>(
    cx: &mut C,
    navigator: &Navigator,
    subscene: ArchiveSubscene,
    archives: &archive::Database,
    reporter: &ScanReporter,
) -> anyhow::Result<()>
where
    C: ScreenCapture + Input + TemplateMatching + Ocr + Clock,
{
    navigator.navigate_to(UiState::Archive(ArchiveState::Detail), cx)?;
    let page = page_type_of(subscene);
    let category = category_id_of(subscene);
    let mut count = 0u32;
    loop {
        count += 1;
        scan_current_item(cx, archives, reporter, page, category, count)?;
        if !advance_to_next_item(cx, count + 1)? {
            break;
        }
    }
    debug!("「下一篇」和「档案详情右箭头」均未找到，扫描完毕（共 {count} 份）");
    navigator.navigate_to(UiState::archive_subscene(subscene), cx)?;
    Ok(())
}

/// OCR 和纠错使用同一份截图，上报时附上该详情画面。
fn scan_current_item<C>(
    cx: &mut C,
    archives: &archive::Database,
    reporter: &ScanReporter,
    page: archive::Page,
    category: archive::Category,
    count: u32,
) -> anyhow::Result<()>
where
    C: ScreenCapture + Ocr,
{
    let screenshot = cx.screenshot()?;
    let ocr_text = read_title(cx, &screenshot, count);
    let corrected = ocr_correction::match_with_correction(
        archives,
        category,
        &ocr_text,
        Some(&ocr_correction::DEFAULT_CORRECTION_OVERRIDES),
    );
    match &corrected {
        Some(c) => info!(
            "第 {} 份档案纠错为：{}（id: {}）",
            count,
            c.title,
            c.item_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None if !ocr_text.is_empty() => info!("第 {count} 份档案标题无法识别"),
        None => {}
    }
    reporter.report(page, category, &screenshot, ocr_text, corrected);
    Ok(())
}

/// OCR 失败按空标题上报，继续扫描下一份档案。
fn read_title<C: Ocr>(cx: &mut C, screenshot: &RgbaImage, count: u32) -> String {
    match cx.recognize_text(screenshot, OCR_ROI) {
        Ok(Some(text)) if !text.trim().is_empty() => {
            info!("第 {} 份档案标题：{}", count, text.trim());
            text.trim().to_string()
        }
        Ok(None | Some(_)) => {
            info!("第 {count} 份档案标题：（空）");
            String::new()
        }
        Err(error) => {
            debug!("OCR 识别失败（第 {count} 份）: {error:#}");
            info!("第 {count} 份档案标题：（OCR 识别失败）");
            String::new()
        }
    }
}

/// 优先点击「下一篇」，其次点击右箭头。匹配或点击失败仍向上传播。
fn advance_to_next_item<C>(cx: &mut C, next_count: u32) -> anyhow::Result<bool>
where
    C: ScreenCapture + TemplateMatching + Input + Clock,
{
    // 上报后重新截图，用当前画面判断是否还能翻页。
    let screenshot = cx.screenshot()?;
    for (label, target) in NEXT_ITEM_TARGETS {
        if let Some(matched) = cx.find_template(&screenshot, &target)? {
            cx.click(matched.region.center().into())?;
            debug!("点击「{label}」，进入第 {next_count} 份档案");
            cx.sleep(Duration::from_millis(200));
            return Ok(true);
        }
    }
    Ok(false)
}

/// 档案标题 OCR 区域，基于 1280 × 720 画面。
pub(crate) const OCR_ROI: Region2D<u32> = ltwh!(350, 58, 578, 42);

const NEXT_ITEM_TARGETS: [(&str, TemplateTarget); 2] = [
    (
        "下一篇",
        TemplateTarget {
            template_name: "情报档案库/下一篇.png",
            roi: ltrb!(762, 654, 925, 711),
            threshold: 0.75,
        },
    ),
    (
        "档案详情右箭头",
        TemplateTarget {
            template_name: "情报档案库/档案详情右箭头.png",
            roi: ltrb!(1206, 313, 1276, 423),
            threshold: 0.75,
        },
    ),
];

/// 六个档案库子界面的完整扫描顺序。
const SCAN_PLAN: &[ArchiveSubscene] = &[
    ArchiveSubscene::Media,
    ArchiveSubscene::Records(RecordsPage::Paper),
    ArchiveSubscene::Records(RecordsPage::Digital),
    ArchiveSubscene::Records(RecordsPage::Collection),
    ArchiveSubscene::Central(CentralPage::Archive),
    ArchiveSubscene::Central(CentralPage::Report),
];

/// 子界面所属的档案库大类 ID（`pageType`：`multi_media` / `text` / `document`）。
fn page_type_of(subscene: ArchiveSubscene) -> archive::Page {
    match subscene {
        ArchiveSubscene::Media => archive::Page::MultiMedia,
        ArchiveSubscene::Records(_) => archive::Page::Text,
        ArchiveSubscene::Central(_) => archive::Page::Document,
    }
}

/// 子界面所属的小类 ID（`categoryId`，与 `prts.json` 中 `allItems` 的
/// `categoryId` 一致）。
fn category_id_of(subscene: ArchiveSubscene) -> archive::Category {
    match subscene {
        ArchiveSubscene::Media => archive::Category::Media,
        ArchiveSubscene::Records(RecordsPage::Paper) => archive::Category::Paper,
        ArchiveSubscene::Records(RecordsPage::Digital) => archive::Category::Digital,
        ArchiveSubscene::Records(RecordsPage::Collection) => archive::Category::Collection,
        ArchiveSubscene::Central(CentralPage::Archive) => archive::Category::Document,
        ArchiveSubscene::Central(CentralPage::Report) => archive::Category::Report,
    }
}

/// 播放扫描提示音（音量取本次设置快照）。
fn play_scan_sound(volume: f32, sound: ScanSound) {
    let app_paths = match AppPaths::new() {
        Ok(app_paths) => app_paths,
        Err(error) => {
            warn!("无法解析扫描提示音资源: {error}");
            return;
        }
    };
    let relative_path = match sound {
        ScanSound::Enable => "sounds/enable.wav",
        ScanSound::Disable => "sounds/disable.wav",
    };
    let path = match app_paths.resolve_resource_file(relative_path) {
        Ok(path) => path,
        Err(error) => {
            warn!("无法解析扫描提示音资源: {error:#}");
            return;
        }
    };
    platform::sound::play_wav(&path, volume);
}

#[derive(Debug, Clone, Copy)]
enum ScanSound {
    Enable,
    Disable,
}
