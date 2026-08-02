use anyhow::Result;
use dak::{
    app::App,
    app_paths::AppPaths,
    hotkey::{HotkeyBinding, HotkeyEvent, HotkeyListener},
    input::{InputBase, SeizeInput},
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::Session,
    tasks::archive_scan::scenes::create_scene_manager,
    window,
};
use rapidocr_core::config::PipelineConfig;
use tracing::info;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_NOREPEAT, VK_DELETE, VK_OEM_1, VK_OEM_7,
};

fn main() -> Result<()> {
    // 解析资源目录（resources/models/logs），不依赖运行时工作目录
    let paths = AppPaths::new()?;

    let _logger_guard = dak::logger::init(&paths.logs_dir);

    window::set_thread_dpi_awareness_context();

    // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台，启动后不自动运行任务）
    let hwnd = window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
    window::ensure_window_on_screen(hwnd)?;

    let client_rect = window::get_client_rect(hwnd)?;
    let resolution = GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
    info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

    // 2. 初始化底层组件（截图、输入、OCR）
    let screencap = Box::new(PrintWindowScreencap::new(hwnd));
    let input = Box::new(SeizeInput::new(hwnd, false));
    let pipeline_config = PipelineConfig::recognition_only();
    let ocr_engine = OcrEngine::new(pipeline_config, &paths.models_dir)?;

    // 3. 注册热键：
    //    - 分号键 `;`（VK_OEM_1）：单次扫描当前档案详情（仅截屏识别）
    //    - 引号键 `'`（VK_OEM_7）：启动 / 停止档案库主任务
    //    - Alt+Delete（VK_DELETE + MOD_ALT）：退出程序（优雅停止后结束脚本）
    let hotkey = HotkeyListener::new(&[
        HotkeyBinding {
            vk: VK_OEM_1.0 as u32,
            modifiers: HOT_KEY_MODIFIERS(0),
            event: HotkeyEvent::ScanSingleArchive,
        },
        HotkeyBinding {
            vk: VK_OEM_7.0 as u32,
            modifiers: HOT_KEY_MODIFIERS(0),
            event: HotkeyEvent::ToggleMainTask,
        },
        HotkeyBinding {
            vk: VK_DELETE.0 as u32,
            modifiers: MOD_ALT | MOD_NOREPEAT,
            event: HotkeyEvent::ExitProgram,
        },
    ])?;
    let stop_flag = hotkey.stop_flag();

    // 4. 创建 Session（传入停止标志，识别 / 输入操作前会检查）
    let session = Session::new(
        hwnd,
        screencap,
        input,
        ocr_engine,
        paths.templates_dir(),
        resolution,
        stop_flag,
    );

    // 5. 构建场景管理器
    let scene_manager = create_scene_manager();

    // 6. 启动应用主事件循环（由热键驱动，不会自动开始运行任务）
    let mut app = App::new(session, scene_manager, hotkey);
    app.run()
}
