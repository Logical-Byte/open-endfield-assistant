//! 连接游戏：组装一次游戏操作所需的 [`Session`]。
//!
//! 程序启动时不依赖游戏窗口；直到开始任务（主任务 / 单次扫描）才获取
//! 游戏窗口、检测分辨率并构建会话。游戏未打开或分辨率不支持时返回错误，
//! 由调用方记录日志而非 panic。

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tracing::info;

use crate::{
    input::SeizeInput,
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::{Session, StopToken},
    window,
};

/// 连接游戏窗口并组装会话。
///
/// # 流程
/// 1. 按标题/类名查找终末地窗口（仅确保在屏幕上，不抢占前台）；
/// 2. 检测客户端分辨率（仅支持 16:9）；
/// 3. 创建截图器与输入器；
/// 4. 组装 [`Session`]（复用共享 OCR 引擎与模板目录）。
pub fn connect_to_game(
    ocr: &Arc<Mutex<OcrEngine>>,
    templates_root: &Path,
    stop: StopToken,
) -> Result<Session> {
    // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台）
    let hwnd = window::get_window_by_title(
        window::ENDFIELD_WINDOW_TITLE,
        Some(window::ENDFIELD_WINDOW_CLASS),
    )?;
    window::ensure_window_on_screen(hwnd)?;

    // 2. 检测分辨率
    let client_rect = window::get_client_rect(hwnd)?;
    let resolution = GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
    info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

    // 3. 创建截图器与输入器
    let screencap = Box::new(PrintWindowScreencap::new(hwnd));
    let input = Box::new(SeizeInput::new(hwnd, false));

    // 4. 组装 Session（复用共享 OCR 引擎与模板目录）
    Ok(Session::new(
        hwnd,
        screencap,
        input,
        Arc::clone(ocr),
        templates_root,
        resolution,
        stop,
    ))
}
