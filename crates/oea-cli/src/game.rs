//! 按命令装配游戏连接、截图与导航能力，不加载无关的识别资源。

use std::{path::PathBuf, thread, time::Duration};

use image::RgbaImage;
use oea_lib::{
    automation::{
        Clock, Input, Key, Point720p, ScreenCapture, TemplateMatch, TemplateMatching,
        TemplateTarget, capabilities, game_environment::ResolutionTransform,
        template_loader::TemplateLoader,
    },
    platform::{
        WindowHandle,
        capture::PrintWindowScreencap,
        input::{Contact, InputBase, SeizeInput},
    },
};
use oea_vision::{ImageRegion, template_matching};
use tracing::debug;

use super::commands;

pub(super) struct Connection {
    window: WindowHandle,
    transform: ResolutionTransform,
}

impl Connection {
    pub(super) fn client_size(&self) -> (u32, u32) {
        self.transform.physical().dimensions()
    }
}

pub(super) fn connect() -> Result<Connection, commands::Error> {
    #[cfg(windows)]
    {
        use oea_lib::{automation::game_environment, platform};

        platform::window::set_thread_dpi_awareness_context();
        let environment = game_environment::connect(game_environment::Requirements::default())
            .map_err(anyhow::Error::new)?;
        let transform =
            ResolutionTransform::new(environment.resolution).map_err(anyhow::Error::new)?;
        Ok(Connection {
            window: environment.window,
            transform,
        })
    }
    #[cfg(not(windows))]
    {
        Err(commands::Error {
            kind: "platform_unsupported",
            source: anyhow::anyhow!(
                "game window commands are unsupported on {}",
                std::env::consts::OS
            ),
        })
    }
}

pub(super) struct Capture {
    screencap: PrintWindowScreencap,
    transform: ResolutionTransform,
}

impl Capture {
    pub(super) fn new(connection: Connection) -> Self {
        Self {
            screencap: PrintWindowScreencap::new(connection.window),
            transform: connection.transform,
        }
    }
}

impl ScreenCapture for Capture {
    fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
        let raw = self.screencap.screencap().map_err(|error| {
            debug!(error = ?error, "游戏截图失败");
            capabilities::Error::CaptureFailed
        })?;
        self.transform
            .to_canonical_image(raw)
            .map_err(capabilities::Error::from)
    }
}

/// 导航器需要的四种能力。截图命令无需创建输入器或模板缓存。
pub(super) struct NavigationContext {
    capture: Capture,
    input: SeizeInput,
    templates: TemplateLoader,
}

impl NavigationContext {
    pub(super) fn new(connection: Connection, templates_root: impl Into<PathBuf>) -> Self {
        let input = SeizeInput::new(connection.window, false);
        Self {
            capture: Capture::new(connection),
            input,
            templates: TemplateLoader::new(templates_root),
        }
    }
}

impl ScreenCapture for NavigationContext {
    fn screenshot(&mut self) -> Result<RgbaImage, capabilities::Error> {
        self.capture.screenshot()
    }
}

impl Input for NavigationContext {
    fn click(&mut self, point: Point720p) -> Result<(), capabilities::Error> {
        let point = self.capture.transform.to_physical(point);
        self.input.click(Contact::Left, point).map_err(|error| {
            debug!(?point, error = ?error, "点击游戏窗口失败");
            capabilities::Error::ExecutionFailed
        })?;
        // 保持生产输入的点击后等待与鼠标归位，避免悬停影响后续识别。
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()
    }

    fn press_key(&mut self, key: Key) -> Result<(), capabilities::Error> {
        let vk_code = match key {
            Key::Escape => 0x1B,
        };
        self.input.press_key(vk_code).map_err(|error| {
            debug!(?key, error = ?error, "发送游戏按键失败");
            capabilities::Error::ExecutionFailed
        })
    }

    fn move_mouse_to_safe_position(&mut self) -> Result<(), capabilities::Error> {
        let point = self
            .capture
            .transform
            .to_physical(Point720p { x: 640, y: 360 });
        self.input
            .touch_move(Contact::Left, point)
            .map_err(|error| {
                debug!(error = ?error, "移动游戏鼠标失败");
                capabilities::Error::ExecutionFailed
            })
    }
}

impl TemplateMatching for NavigationContext {
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>, capabilities::Error> {
        let matched = (|| -> anyhow::Result<template_matching::MatchResult> {
            template_matching::match_template(
                &ImageRegion::new(screenshot, target.roi.into())?,
                self.templates.get(target.template_name)?,
            )
        })()
        .map_err(|error| {
            debug!(template = target.template_name, roi = ?target.roi, error = ?error, "模板匹配失败");
            capabilities::Error::ExecutionFailed
        })?;
        Ok(
            (matched.score >= target.threshold).then_some(TemplateMatch {
                region: matched.region.into(),
                score: matched.score,
            }),
        )
    }
}

impl Clock for NavigationContext {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}
