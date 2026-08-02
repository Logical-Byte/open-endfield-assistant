//! 会话工厂模块。
//!
//! 程序启动时不依赖游戏窗口：即使终末地尚未打开，GUI 也能正常启动。
//! 直到开始任务（启动主任务 / 单次扫描）时才获取游戏窗口、检测分辨率并构建
//! [`Session`]；游戏未打开或分辨率不支持时返回错误，由调用方记录日志而非 panic。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use tracing::info;

use crate::{
    input::{InputBase, SeizeInput},
    ocr::OcrEngine,
    resolution::GameResolution,
    screencap::PrintWindowScreencap,
    session::Session,
    window,
};

/// 会话工厂：聚合创建 [`Session`] 所需的不依赖窗口的零件，
/// 在任务开始时获取窗口并组装出完整的会话。
pub struct SessionFactory {
    /// 共享 OCR 引擎（跨任务复用，避免重复加载模型）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 模板图片根目录
    templates_root: PathBuf,
    /// 停止标志（来自热键监听器）
    stop_flag: Arc<AtomicBool>,
}

impl SessionFactory {
    /// 创建会话工厂。
    ///
    /// # 参数
    /// - `ocr`: 已初始化的 OCR 引擎（不依赖游戏窗口）
    /// - `templates_root`: 模板图片根目录（如 [`crate::app_paths::AppPaths::templates_dir`]）
    /// - `stop_flag`: 热键停止标志，每次操作前检查
    pub fn new(
        ocr: Arc<Mutex<OcrEngine>>,
        templates_root: impl Into<PathBuf>,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            ocr,
            templates_root: templates_root.into(),
            stop_flag,
        }
    }

    /// 获取游戏窗口并构建 [`Session`]（任务开始时调用）。
    ///
    /// 流程：获取窗口句柄 → 确保窗口在屏幕上 → 检测分辨率（仅支持 16:9）
    /// → 创建截图器与输入器 → 组装 Session。
    ///
    /// 游戏未打开或分辨率不支持时返回错误，由调用方（如 `App::start_scan`）
    /// 记录日志并提示用户，而不是使程序 panic。
    pub fn build_session(&self) -> Result<Session> {
        // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台）
        let hwnd = window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
        window::ensure_window_on_screen(hwnd)?;

        let client_rect = window::get_client_rect(hwnd)?;
        let resolution =
            GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
        info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

        // 2. 初始化截图器与输入器
        let screencap = Box::new(PrintWindowScreencap::new(hwnd));
        let input = Box::new(SeizeInput::new(hwnd, false));

        // 3. 组装 Session（复用共享 OCR 引擎与模板目录）
        let session = Session::new(
            hwnd,
            screencap,
            input,
            self.ocr.clone(),
            &self.templates_root,
            resolution,
            self.stop_flag.clone(),
        );
        Ok(session)
    }
}
