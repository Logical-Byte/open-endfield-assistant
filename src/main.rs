use anyhow::Result;
use dak::{
    hotkey::HotkeyListener,
    input::{InputBase, SeizeInput},
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::Session,
    task::TaskRunner,
    tasks::archive_scan::{ArchiveScanTask, scenes::create_scene_manager},
};
use rapidocr_core::config::PipelineConfig;
use tracing::info;

fn main() -> Result<()> {
    let _logger_guard = dak::logger::init();

    dak::set_thread_dpi_awareness_context();

    // 1. 获取游戏窗口
    let hwnd = dak::window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
    dak::window::ensure_foreground_and_topmost(hwnd)?;
    dak::window::ensure_window_on_screen(hwnd)?;

    let client_rect = dak::window::get_client_rect(hwnd)?;
    let resolution = GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
    info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

    // 2. 初始化底层组件
    let screencap = Box::new(PrintWindowScreencap::new(hwnd));
    let input = Box::new(SeizeInput::new(hwnd, false));
    let pipeline_config = PipelineConfig::recognition_only();
    let ocr_engine = OcrEngine::new(pipeline_config)?;

    // 3. 注册快捷键（在创建 Session 之前，因为 Session 需要持有停止标志）
    let hotkey = HotkeyListener::alt_delete();
    info!("按 Alt+Delete 停止");
    let stop_flag = hotkey.stop_flag();

    // 4. 创建 Session（传入停止标志，操作前会检查）
    let mut session = Session::new(hwnd, screencap, input, ocr_engine, resolution, stop_flag);

    // 5. 创建 SceneManager 和 TaskRunner
    let scene_manager = create_scene_manager();
    let mut task_runner = TaskRunner::new(scene_manager);

    // 6. 运行扫描档案库任务（Session 内每次操作前检查 stop_flag）
    let task = ArchiveScanTask;
    let result = task_runner.run_task(&task, &mut session);

    if hotkey.stop_requested() {
        info!("收到停止信号 (Alt+Delete)，任务已中断");
    }

    result?; // 如果任务本身报错，也传播出去
    Ok(())
}
