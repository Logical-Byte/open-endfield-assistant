//! 连接游戏：组装一次游戏操作所需的 [`Session`]。
//!
//! 程序启动时不依赖游戏窗口；直到开始任务才获取游戏窗口、检测分辨率与
//! HDR 环境并构建会话。游戏未打开、分辨率不支持或显示器开启 HDR 时返回错误，
//! 由调用方记录日志而非 panic。

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use tracing::{info, warn};

use crate::{
    input::SeizeInput,
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::{Session, StopToken},
    window::{self, hdr},
};

/// 连接游戏窗口并组装会话。
///
/// # 流程
/// 1. 按标题/类名查找终末地窗口，若被最小化则恢复（仅确保在屏幕上，不抢占前台）；
/// 2. 检测客户端分辨率（仅支持 16:9）；
/// 3. 检查终末地所在显示器是否开启 HDR（开启会致截图颜色失真、影响识别，拒绝执行）；
/// 4. 创建截图器与输入器；
/// 5. 组装 [`Session`]（复用共享 OCR 引擎与模板目录）。
pub fn connect_to_game(
    ocr: &Arc<Mutex<OcrEngine>>,
    templates_root: &Path,
    stop: StopToken,
) -> Result<Session> {
    // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台）
    let hwnd = window::get_window_by_title(
        Some(window::ENDFIELD_WINDOW_CLASS),
        Some(window::ENDFIELD_WINDOW_TITLE),
    )
    .context("未找到终末地窗口，请先打开游戏")?;
    // 若窗口被最小化则先恢复，否则 `ensure_window_on_screen` 会跳过调整
    let _ = window::restore_window_if_minimized(hwnd).inspect_err(|e| warn!("恢复窗口失败: {e:#}"));
    let _ =
        window::ensure_window_on_screen(hwnd).inspect_err(|e| warn!("确保窗口在屏幕上失败: {e:#}"));

    // 2. 检测分辨率
    let client_rect = window::get_client_rect(hwnd)?;
    let resolution = GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
    info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

    // 3. 检查终末地所在显示器是否开启 HDR（开启会致截图颜色失真、影响识别，拒绝执行）
    match hdr::is_hdr_enabled_on_window_monitor(hwnd) {
        Ok(true) => {
            bail!("终末地所在显示器已开启 HDR，截图颜色会失真导致识别异常，请关闭 HDR 后重试")
        }
        Ok(false) => {}
        Err(e) => warn!("检查显示器 HDR 状态失败: {e:#}，继续执行任务"),
    }

    // 4. 创建截图器与输入器
    let screencap = Box::new(PrintWindowScreencap::new(hwnd));
    let input = Box::new(SeizeInput::new(hwnd, false));

    // 5. 组装 Session（复用共享 OCR 引擎与模板目录）
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
