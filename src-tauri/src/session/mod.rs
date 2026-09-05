//! 游戏会话。
//!
//! 连接时发现游戏窗口、验证运行环境并组装基础设施组件；运行时由
//! [`AutomationContext`] 借用这些组件，并翻译工作流的 720p 基准能力调用。
//!
//! 职责边界：
//! - ✅ 连接游戏窗口，检查分辨率与 HDR 环境并创建会话；
//! - ✅ 持有基础设施状态，**不含任何业务逻辑**（不识别场景、不导航、不扫描）；
//! - ✅ **不依赖前端**（结果上报由任务层通过 [`crate::task::archive_scan::ScanReporter`] 完成）；
//! - ✅ 只依赖基础设施层。
//!
//! 会话贯穿一次游戏操作（扫描档案库任务），由调用方以 `&mut` 串行使用。

mod automation_context;

pub use automation_context::AutomationContext;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use tracing::{info, warn};

use crate::{
    ocr::OcrEngine,
    resolution::GameResolution,
    task::TaskStopped,
    template_matching::LazyTemplateLoader,
    windows_ops::{
        self, WindowHandle,
        capture::{PrintWindowScreencap, ScreencapBase},
        input::{InputBase, SeizeInput},
    },
};

/// 停止令牌：热键 / 命令通过它请求中断，Session 每次操作前轮询。
pub type StopToken = Arc<AtomicBool>;

/// # Send 安全性
/// `Session` 持有非拥有型窗口句柄，不自动 `Send`。
/// 窗口句柄在 OS 层面对线程无亲和性，且本类型始终由调用方以 `&mut`
/// 串行使用（同一时刻仅一个线程访问），因此跨线程移动是安全的。
/// 若未来有人破坏"单线程串行使用"这一前提，本 unsafe 承诺即失效。
unsafe impl Send for Session {}

pub struct Session {
    /// 游戏窗口句柄（前台判定 / 日志用）
    pub hwnd: WindowHandle,
    /// 游戏实际分辨率（仅支持 16:9）
    pub resolution: GameResolution,
    /// 截图器（可运行时替换，扩展点）
    screencap: Box<dyn ScreencapBase>,
    /// 输入器（可运行时替换，扩展点）
    input: Box<dyn InputBase>,
    /// 共享 OCR 引擎（跨会话复用模型加载）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 模板加载器（懒加载 + 缓存）
    templates: LazyTemplateLoader,
    /// 停止令牌
    stop: StopToken,
}

impl Session {
    /// 连接游戏窗口并创建会话。
    ///
    /// # 流程
    /// 1. 按标题/类名查找终末地窗口，若被最小化则恢复（仅确保在屏幕上，不抢占前台）；
    /// 2. 检测客户端分辨率（仅支持 16:9）；
    /// 3. 检查终末地所在显示器是否开启 HDR（开启会致截图颜色失真、影响识别，拒绝执行）；
    /// 4. 创建截图器与输入器；
    /// 5. 组装会话（复用共享 OCR 引擎与模板目录）。
    pub(crate) fn connect(
        ocr: &Arc<Mutex<OcrEngine>>,
        templates_root: &Path,
        stop: StopToken,
    ) -> Result<Self> {
        // 1. 获取游戏窗口（仅确保窗口在屏幕上，不抢占前台）
        let hwnd = windows_ops::window::get_window_by_title(
            Some(windows_ops::window::ENDFIELD_WINDOW_CLASS),
            Some(windows_ops::window::ENDFIELD_WINDOW_TITLE),
        )
        .context("未找到终末地窗口，请先打开游戏")?;
        // 若窗口被最小化则先恢复，否则 `ensure_window_on_screen` 会跳过调整
        let _ = windows_ops::window::restore_window_if_minimized(hwnd)
            .inspect_err(|e| warn!("恢复窗口失败: {e:#}"));
        let _ = windows_ops::window::ensure_window_on_screen(hwnd)
            .inspect_err(|e| warn!("确保窗口在屏幕上失败: {e:#}"));

        // 2. 检测分辨率
        let client_rect = windows_ops::window::get_client_rect(hwnd)?;
        let resolution =
            GameResolution::new(client_rect.width() as u32, client_rect.height() as u32)?;
        info!("游戏分辨率: {}×{}", resolution.width, resolution.height);

        // 3. 检查终末地所在显示器是否开启 HDR（开启会致截图颜色失真、影响识别，拒绝执行）
        match windows_ops::window::hdr::is_hdr_enabled_on_window_monitor(hwnd) {
            Ok(true) => {
                bail!("终末地所在显示器已开启 HDR，截图颜色会失真导致识别异常，请关闭 HDR 后重试")
            }
            Ok(false) => {}
            Err(e) => warn!("检查显示器 HDR 状态失败: {e:#}，继续执行任务"),
        }

        // 4. 创建截图器与输入器
        let screencap = Box::new(PrintWindowScreencap::new(hwnd));
        let input = Box::new(SeizeInput::new(hwnd, false));

        // 5. 组装 `Session`（复用共享 OCR 引擎与模板目录）
        Ok(Self::new(
            hwnd,
            screencap,
            input,
            Arc::clone(ocr),
            templates_root,
            resolution,
            stop,
        ))
    }

    /// 使用已组装的依赖创建会话。
    #[allow(clippy::too_many_arguments)]
    fn new(
        hwnd: WindowHandle,
        screencap: Box<dyn ScreencapBase>,
        input: Box<dyn InputBase>,
        ocr: Arc<Mutex<OcrEngine>>,
        templates_root: impl Into<PathBuf>,
        resolution: GameResolution,
        stop: StopToken,
    ) -> Self {
        Self {
            hwnd,
            screencap,
            input,
            ocr,
            templates: LazyTemplateLoader::new(templates_root),
            resolution,
            stop,
        }
    }

    // ========== 停止 ==========

    /// 检查是否收到停止信号，收到则返回 [`TaskStopped`] 中断执行。
    ///
    /// 停止不是"任务出错"：上层用 `downcast_ref::<TaskStopped>()` 区分。
    fn check_stop(&self) -> Result<()> {
        if self.stop.load(Ordering::Relaxed) {
            Err(TaskStopped.into())
        } else {
            Ok(())
        }
    }

    /// 清除停止信号（启动任务前调用，避免上次残留误伤后续操作）。
    pub fn reset_stop(&mut self) {
        self.stop.store(false, Ordering::Relaxed);
    }

    /// 借用会话组件，构造工作流使用的生产自动化上下文。
    pub fn automation_context(&mut self) -> AutomationContext<'_> {
        AutomationContext::new(self)
    }

    // ========== 截图器 / 输入器切换（扩展点） ==========

    /// 替换截图器（运行时切换，为未来多种截图器铺路）。
    pub fn set_screencap(&mut self, screencap: Box<dyn ScreencapBase>) {
        self.screencap = screencap;
    }

    /// 替换输入器（运行时切换，为未来多种输入器铺路）。
    pub fn set_input(&mut self, input: Box<dyn InputBase>) {
        self.input = input;
    }
}
