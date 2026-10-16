//! 真实档案扫描的工作线程和完整执行流程。

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use image::RgbaImage;
use tracing::{debug, info, warn};

use crate::{
    app_paths::AppPaths,
    automation::{
        self, Clock, EventSink, Input, Ocr, ScreenCapture, StopToken, TemplateMatching,
        TemplateTarget, capabilities,
        runtime::{self, WorkerExit},
        session::Session,
        stats::counts::{Capture, CaptureSummary},
    },
    data::{AppData, archive},
    navigation::{ArchiveState, ArchiveSubscene, CentralPage, Navigator, RecordsPage, UiState},
    platform, settings,
    utils::region::{Region2D, ltrb, ltwh},
    vision,
};

use super::{matching, reporting::ScanReporter};

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
        let exit = WorkerExit { result, capture };
        let sound = match &exit.result {
            Ok(()) => ScanSound::Enable,
            Err(_) => ScanSound::Disable,
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
) -> Result<(), automation::WorkerError> {
    // 连接游戏可能耗时，所以留在工作线程中。
    let mut session = Session::connect(&worker.ocr, stop)?;

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
    result
}

/// 移动鼠标、进入档案库主界面，然后按顺序扫描全部档案分类对应的子界面。
fn scan_archives<C>(
    cx: &mut C,
    navigator: &Navigator,
    archives: &archive::Database,
    reporter: &ScanReporter,
) -> Result<(), automation::WorkerError>
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

/// 从当前档案分类对应的子界面进入档案详情页面，逐份扫描，完成后返回该子界面。
fn scan_subscene<C>(
    cx: &mut C,
    navigator: &Navigator,
    subscene: ArchiveSubscene,
    archives: &archive::Database,
    reporter: &ScanReporter,
) -> Result<(), automation::WorkerError>
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
) -> Result<(), automation::WorkerError>
where
    C: ScreenCapture + Ocr,
{
    let screenshot = cx.screenshot()?;
    let ocr_text = read_title(cx, &screenshot, count)?;
    let (result, body_ocr_result) = match matching::match_title(archives, category, &ocr_text) {
        matching::TitleMatch::Complete(result) => (result, None),
        matching::TitleMatch::NeedsBody(context) => {
            // 继续使用标题截图，避免两次采集之间翻页导致标题和正文来自不同档案。
            let body = read_body(cx, &screenshot, context.ocr_roi(), count)?;
            (context.resolve(body.as_deref()), body)
        }
    };
    match &result {
        matching::MatchResult::Exact { archive_id } => {
            info!("第 {count} 份档案精确匹配：{archive_id}")
        }
        matching::MatchResult::Recommendations { archive_ids } => {
            info!("第 {count} 份档案待核对，推荐 {} 个候选", archive_ids.len())
        }
    }
    reporter.report(
        page,
        category,
        &screenshot,
        ocr_text,
        body_ocr_result,
        result,
    );
    Ok(())
}

/// OCR 失败按空标题上报，继续扫描下一份档案。
fn read_title<C: Ocr>(
    cx: &mut C,
    screenshot: &RgbaImage,
    count: u32,
) -> Result<String, capabilities::Error> {
    match cx.recognize_text(screenshot, OCR_ROI) {
        Ok(Some(text)) if !text.trim().is_empty() => {
            info!("第 {} 份档案标题：{}", count, text.trim());
            Ok(text.trim().to_string())
        }
        Ok(None | Some(_)) => {
            info!("第 {count} 份档案标题：（空）");
            Ok(String::new())
        }
        Err(error) if error.is_stopped_by_user() => Err(error),
        Err(error) => {
            debug!("OCR 识别失败（第 {count} 份）: {error:#}");
            info!("第 {count} 份档案标题：（OCR 识别失败）");
            Ok(String::new())
        }
    }
}

/// 按需读取正文并保留原始观察。普通 OCR 失败只放弃正文确认，用户停止仍立即传播。
fn read_body<C: Ocr>(
    cx: &mut C,
    screenshot: &RgbaImage,
    roi: Region2D<u32>,
    count: u32,
) -> Result<Option<String>, capabilities::Error> {
    match cx.recognize_text(screenshot, roi) {
        Ok(body) => {
            debug!(count, body = ?body, "读取同名档案正文");
            Ok(body)
        }
        Err(error) if error.is_stopped_by_user() => Err(error),
        Err(error) => {
            debug!(count, error = ?error, "正文 OCR 识别失败，保留标题推荐");
            Ok(None)
        }
    }
}

/// 优先点击「下一篇」，其次点击右箭头。匹配或点击失败仍向上传播。
fn advance_to_next_item<C>(cx: &mut C, next_count: u32) -> Result<bool, automation::WorkerError>
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

/// 子界面所属的档案页面 ID（`pageType`：`multi_media` / `text` / `document`）。
fn page_type_of(subscene: ArchiveSubscene) -> archive::Page {
    match subscene {
        ArchiveSubscene::Media => archive::Page::MultiMedia,
        ArchiveSubscene::Records(_) => archive::Page::Text,
        ArchiveSubscene::Central(_) => archive::Page::Document,
    }
}

/// 子界面所属的档案分类 ID（`categoryId`，与 `prts.json` 中 `allItems` 的
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

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use image::RgbaImage;

    use crate::{
        app_paths::AppPaths,
        automation::{
            self, Clock, Event, Input, Key, Ocr, Point720p, ScreenCapture, TemplateMatch,
            TemplateMatching, TemplateTarget, capabilities, events::testing::RecordingEventSink,
        },
        data::archive,
        navigation::{Navigator, UiState},
        utils::region::Region2D,
    };

    /// 替代窗口截图和 OCR 系统能力，测试仍执行真实单项扫描、匹配与事件上报。
    struct PageObservation {
        texts: std::collections::VecDeque<Result<Option<String>, capabilities::Error>>,
        captures: usize,
    }

    impl ScreenCapture for PageObservation {
        fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
            self.captures += 1;
            Ok(RgbaImage::from_pixel(
                1280,
                720,
                image::Rgba([20, 30, 40, 255]),
            ))
        }
    }

    impl Ocr for PageObservation {
        fn recognize_text(
            &mut self,
            screenshot: &RgbaImage,
            _: Region2D<u32>,
        ) -> Result<Option<String>, capabilities::Error> {
            assert_eq!(screenshot.get_pixel(0, 0).0, [20, 30, 40, 255]);
            self.texts.pop_front().expect("不应执行额外 OCR")
        }
    }

    fn scan_observation(
        title: &str,
        body: Option<Result<Option<String>, capabilities::Error>>,
        category: archive::Category,
    ) -> (
        Result<(), automation::WorkerError>,
        Vec<Event>,
        PageObservation,
    ) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let index = archive::Database::load(&AppPaths::with_root_dir(root)).unwrap();
        let sink = Arc::new(RecordingEventSink::default());
        let event_sink = Arc::clone(&sink);
        let reporter = super::ScanReporter::new(event_sink);
        let mut cx = PageObservation {
            texts: std::iter::once(Ok(Some(title.into())))
                .chain(body)
                .collect(),
            captures: 0,
        };
        let result =
            super::scan_current_item(&mut cx, &index, &reporter, archive::Page::Text, category, 1);
        let events = sink.wait_for_events(usize::from(result.is_ok()));
        (result, events, cx)
    }

    #[test]
    fn known_duplicate_reads_body_from_same_frame_and_reports_original_observation() {
        let body = " 保佑寨子■■■■ ";
        let (result, events, cx) = scan_observation(
            "挂在竹子上的字条",
            Some(Ok(Some(body.into()))),
            archive::Category::Paper,
        );
        result.unwrap();
        let Event::ArchiveItemScanned(item) = &events[0] else {
            panic!("应上报扫描记录")
        };
        assert_eq!(
            item.matching,
            super::matching::MatchResult::Exact {
                archive_id: archive::ArchiveId::new("nar_paper_map02_93_2"),
            }
        );
        assert_eq!(item.body_ocr_result.as_deref(), Some(body));
        assert_eq!(cx.captures, 1);
        assert!(cx.texts.is_empty());
    }

    #[test]
    fn title_only_results_skip_body_ocr() {
        for (title, category) in [
            ("泰拉", archive::Category::Document),
            ("挂在竹子上的纸条", archive::Category::Paper),
            ("", archive::Category::Paper),
        ] {
            // 未准备正文返回值，多余的 OCR 请求会使测试失败。
            let (result, events, cx) = scan_observation(title, None, category);
            result.unwrap();
            let Event::ArchiveItemScanned(item) = &events[0] else {
                panic!("应上报扫描记录")
            };
            assert!(item.body_ocr_result.is_none());
            assert!(
                serde_json::to_value(item)
                    .unwrap()
                    .get("bodyOcrResult")
                    .is_none()
            );
            assert_eq!(cx.captures, 1);
        }
    }

    #[test]
    fn unavailable_body_preserves_recommendations_but_user_stop_aborts_reporting() {
        for body in [
            Ok(None),
            Ok(Some(String::new())),
            Err(capabilities::Error::ExecutionFailed),
        ] {
            let (result, events, _) =
                scan_observation("挂在竹子上的字条", Some(body), archive::Category::Paper);
            result.unwrap();
            let Event::ArchiveItemScanned(item) = &events[0] else {
                panic!("应上报扫描记录")
            };
            assert_eq!(
                item.matching,
                super::matching::MatchResult::Recommendations {
                    archive_ids: vec![
                        archive::ArchiveId::new("nar_paper_map02_93_1"),
                        archive::ArchiveId::new("nar_paper_map02_93_2")
                    ],
                }
            );
        }
        let (result, events, _) = scan_observation(
            "挂在竹子上的字条",
            Some(Err(capabilities::Error::StoppedByUser)),
            archive::Category::Paper,
        );
        assert!(result.unwrap_err().is_stopped_by_user());
        assert!(events.is_empty());
    }

    struct FailedCapture(Option<capabilities::Error>);

    impl ScreenCapture for FailedCapture {
        fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
            Err(self.0.take().unwrap())
        }
    }

    impl Input for FailedCapture {
        fn click(&mut self, _: Point720p) -> Result<(), capabilities::Error> {
            unreachable!()
        }
        fn press_key(&mut self, _: Key) -> Result<(), capabilities::Error> {
            unreachable!()
        }
        fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
            unreachable!()
        }
    }

    impl TemplateMatching for FailedCapture {
        fn find_template(
            &mut self,
            _: &RgbaImage,
            _: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>, capabilities::Error> {
            unreachable!()
        }
    }

    impl Clock for FailedCapture {
        fn sleep(&mut self, _: Duration) {
            unreachable!()
        }
    }

    fn navigate(error: capabilities::Error) -> automation::WorkerError {
        Navigator::new()
            .navigate_to(UiState::Terminal, &mut FailedCapture(Some(error)))
            .unwrap_err()
            .into()
    }

    #[test]
    fn navigation_stop_finishes_without_a_failure() {
        assert!(navigate(capabilities::Error::StoppedByUser).is_stopped_by_user());
    }

    #[test]
    fn navigation_preserves_actionable_reasons_without_serializing_diagnostics() {
        for (error, expected) in [
            (
                automation::GameEnvironmentError::WindowUnavailable.into(),
                serde_json::json!({
                    "status": "failed", "error": {
                        "scope": "worker", "kind": "capability",
                        "reason": { "kind": "gameEnvironment", "reason": { "kind": "windowUnavailable" } }
                    }
                }),
            ),
            (
                capabilities::Error::CaptureFailed,
                serde_json::json!({
                    "status": "failed", "error": { "scope": "worker", "kind": "capability", "reason": { "kind": "captureFailed" } }
                }),
            ),
        ] {
            let error = navigate(error);
            let outcome = automation::RunOutcome::Failed {
                error: error.into(),
            };
            assert_eq!(serde_json::to_value(outcome).unwrap(), expected);
        }
    }
}
