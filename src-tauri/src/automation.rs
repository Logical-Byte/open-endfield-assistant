//! 工作流使用的游戏自动化能力边界。
//!
//! 档案扫描、场景导航和路由执行只依赖本模块中的细粒度 trait，避免了解
//! [`Session`](crate::session::Session) 如何持有窗口、截图器、输入器和识别资源。
//! 生产环境由 [`AutomationContext`](crate::session::AutomationContext) 把这些请求
//! 翻译到现有会话组件；测试可以直接实现相同的能力接口。

use std::time::Duration;

use anyhow::Result;
use image::RgbaImage;

use crate::utils::{point::Point2D, region::Region2D};

/// 1280x720 基准坐标系中的点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point720p {
    /// 相对于 1280 像素宽基准画布的横坐标。
    pub x: u32,
    /// 相对于 720 像素高基准画布的纵坐标。
    pub y: u32,
}

impl From<Point2D<u32>> for Point720p {
    fn from(point: Point2D<u32>) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

/// 工作流可表达的逻辑按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Escape 键。
    Escape,
}

/// 一次模板搜索的逻辑目标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemplateTarget {
    /// 模板资源的逻辑名称。
    pub template_name: &'static str,
    /// 720p 基准截图中的搜索区域。
    pub roi: Region2D<u32>,
    /// 视为匹配成功的最低分数。
    pub threshold: f32,
}

/// 工作流可见的模板匹配结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemplateMatch {
    /// 匹配区域在 720p 基准截图中的位置。
    pub region: Region2D<u32>,
    /// 模板匹配分数。
    pub score: f32,
}

/// 为需要显式获取识别帧的工作流提供截图能力。
pub trait ScreenCapture {
    /// 获取一张缩放到 1280x720 基准分辨率的新截图。
    fn screenshot(&mut self) -> Result<RgbaImage>;
}

/// 为工作流提供基于 720p 坐标和逻辑按键的输入能力。
pub trait Input {
    /// 点击 720p 基准坐标，并执行生产输入器约定的鼠标归位。
    fn click(&mut self, point: Point720p) -> Result<()>;
    /// 按下并松开一个逻辑按键。
    fn press_key(&mut self, key: Key) -> Result<()>;
    /// 将鼠标移到不会干扰后续识别的安全位置。
    fn move_mouse_to_safe_position(&mut self) -> Result<()>;
}

/// 在工作流提供的截图中查找模板。
///
/// `Ok(None)` 是目标不存在这一业务结果；模板加载或匹配失败通过 `Err` 传播。
pub trait TemplateMatching {
    /// 在目标区域内查找模板，并保留匹配位置与分数。
    fn find_template(
        &mut self,
        screenshot: &RgbaImage,
        target: &TemplateTarget,
    ) -> Result<Option<TemplateMatch>>;
}

/// 为需要读取界面文字的工作流提供 OCR 能力。
pub trait Ocr {
    /// 识别截图区域中的单行文字；没有检测到文字时返回 `Ok(None)`。
    fn recognize_text(
        &mut self,
        screenshot: &RgbaImage,
        region: Region2D<u32>,
    ) -> Result<Option<String>>;
}

/// 为声明式导航和业务工作流提供可替换的计时能力。
pub trait Clock {
    /// 阻塞当前工作流指定时长。
    fn sleep(&mut self, duration: Duration);
}
