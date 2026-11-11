//! 游戏会话。
//!
//! 实现工作流所需的游戏操作能力，并封装游戏窗口、输入、截图与识别资源。

mod capability_impls;

use std::sync::{Arc, Mutex};

use oea_vision as vision;
use tracing::info;

use crate::automation::{
    StopToken, capabilities,
    game_environment::{self, ResolutionTransform},
    is_stop_requested,
    template_loader::TemplateLoader,
};

use oea_window::{InputPolicy, Window};

pub struct Session {
    /// 游戏窗口句柄（前台判定 / 日志用）
    pub window: Window,
    /// 720p 识别坐标与游戏窗口物理坐标之间的转换
    resolution_transform: ResolutionTransform,
    /// 绑定游戏窗口和默认策略的输入器。
    input: oea_window::Input,
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
        let environment = game_environment::connect(game_environment::ConnectionMode::Automation)?;
        info!("游戏分辨率: {}", environment.resolution);
        let window = environment.window;
        let resolution_transform = environment.transform;

        let input = InputPolicy::default().bind(window);

        Ok(Self {
            window,
            input,
            ocr: Arc::clone(ocr),
            templates,
            resolution_transform,
            stop,
        })
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
