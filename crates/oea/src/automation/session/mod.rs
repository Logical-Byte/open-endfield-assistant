//! 游戏会话。
//!
//! 实现工作流所需的游戏操作能力，并封装游戏窗口、输入、截图与识别资源。

mod capability_impls;

use std::sync::{Arc, Mutex};

use oea_vision as vision;
use tracing::info;

use crate::{
    automation::{
        StopToken, capabilities,
        game_environment::{self, ResolutionTransform},
        is_stop_requested,
        template_loader::TemplateLoader,
    },
    platform::{
        WindowHandle,
        capture::{PrintWindowScreencap, ScreencapBase},
        input::{InputBase, SeizeInput},
    },
};

/// # Send 安全性
/// `Session` 持有非拥有型窗口句柄，不自动 `Send`。
/// 窗口句柄在 OS 层面对线程无亲和性，且本类型始终由调用方以 `&mut`
/// 串行使用（同一时刻仅一个线程访问），因此跨线程移动是安全的。
/// 若未来有人破坏"单线程串行使用"这一前提，本 unsafe 承诺即失效。
unsafe impl Send for Session {}

pub struct Session {
    /// 游戏窗口句柄（前台判定 / 日志用）
    pub hwnd: WindowHandle,
    /// 720p 识别坐标与游戏窗口物理坐标之间的转换
    resolution_transform: ResolutionTransform,
    /// 截图器
    screencap: Box<dyn ScreencapBase>,
    /// 输入器
    input: Box<dyn InputBase>,
    /// 共享 OCR 引擎（跨会话复用模型加载）
    ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
    /// 模板加载器（懒加载 + 缓存）
    templates: TemplateLoader,
    /// 停止令牌
    stop: StopToken,
}

impl Session {
    /// 连接游戏窗口并创建会话。
    pub fn connect(
        ocr: &Arc<Mutex<vision::ocr::OcrEngine>>,
        templates: TemplateLoader,
        stop: StopToken,
    ) -> Result<Self, capabilities::Error> {
        let environment = game_environment::connect(game_environment::Requirements::default())?;
        info!("游戏分辨率: {}", environment.resolution);
        let hwnd = environment.window;
        let resolution_transform = ResolutionTransform::new(environment.resolution)?;

        let screencap = Box::new(PrintWindowScreencap::new(hwnd));
        let input = Box::new(SeizeInput::new(hwnd, false));

        Ok(Self::new(
            hwnd,
            screencap,
            input,
            Arc::clone(ocr),
            templates,
            resolution_transform,
            stop,
        ))
    }

    /// 使用已组装的依赖创建会话。
    #[allow(clippy::too_many_arguments)]
    fn new(
        hwnd: WindowHandle,
        screencap: Box<dyn ScreencapBase>,
        input: Box<dyn InputBase>,
        ocr: Arc<Mutex<vision::ocr::OcrEngine>>,
        templates: TemplateLoader,
        resolution_transform: ResolutionTransform,
        stop: StopToken,
    ) -> Self {
        Self {
            hwnd,
            screencap,
            input,
            ocr,
            templates,
            resolution_transform,
            stop,
        }
    }

    // ========== 停止 ==========

    /// 检查停止信号，由工作流出口将 `StoppedByUser` 转换为正常终态。
    pub(super) fn check_stop(&self) -> Result<(), capabilities::Error> {
        if is_stop_requested(&self.stop) {
            Err(capabilities::Error::StoppedByUser)
        } else {
            Ok(())
        }
    }
}
