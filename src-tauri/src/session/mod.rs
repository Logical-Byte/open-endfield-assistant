//! 游戏会话（纯门面）。
//!
//! 把基础设施层的原始能力统一翻译成 **720p 基准** 的任务 API：
//! - 识别操作：先截图并缩放到 1280×720 基准；
//! - 输入操作：先把 720p 坐标缩放到实际分辨率。
//!
//! 职责边界：
//! - ✅ 只做薄委托与坐标缩放，**不含任何业务逻辑**（不识别场景、不导航、不扫描）；
//! - ✅ **不依赖前端**（结果上报由任务层通过 [`crate::tasks::archive_scan::ScanReporter`] 完成）；
//! - ✅ 只依赖基础设施层。
//!
//! 会话贯穿一次游戏操作（扫描档案库任务），由调用方以 `&mut` 串行使用。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use image::{DynamicImage, RgbaImage, imageops};
use imageproc::contrast::ThresholdType;
use windows::Win32::Foundation::HWND;

use crate::{
    input::{Contact, InputBase},
    ocr::{OcrEngine, text_detection},
    resolution::GameResolution,
    screencap::ScreencapBase,
    task::TaskStopped,
    template_matching::{MatchResult, TemplateManager},
    utils::{point::Point2D, region::Region2D},
};

/// 停止令牌：热键 / 命令通过它请求中断，Session 每次操作前轮询。
pub type StopToken = Arc<AtomicBool>;

/// 游戏会话：一次游戏操作（扫描档案库任务）的统一上下文。
///
/// # Send 安全性
/// `Session` 持有 Win32 句柄（`HWND` 内部为裸指针），不自动 `Send`。
/// 窗口句柄在 OS 层面对线程无亲和性，且本类型始终由调用方以 `&mut`
/// 串行使用（同一时刻仅一个线程访问），因此跨线程移动是安全的。
/// 若未来有人破坏"单线程串行使用"这一前提，本 unsafe 承诺即失效。
unsafe impl Send for Session {}

pub struct Session {
    /// 游戏窗口句柄（前台判定 / 日志用）
    pub hwnd: HWND,
    /// 游戏实际分辨率（仅支持 16:9）
    pub resolution: GameResolution,
    /// 截图器（可运行时替换，扩展点）
    screencap: Box<dyn ScreencapBase>,
    /// 输入器（可运行时替换，扩展点）
    input: Box<dyn InputBase>,
    /// 共享 OCR 引擎（跨会话复用模型加载）
    ocr: Arc<Mutex<OcrEngine>>,
    /// 模板匹配管理器（懒加载 + 缓存）
    templates: TemplateManager,
    /// 停止令牌
    stop: StopToken,
}

impl Session {
    /// 创建会话（由 [`crate::connect::connect_to_game`] 组装）。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hwnd: HWND,
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
            templates: TemplateManager::new(templates_root),
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

    // ========== 截图（统一 720p 基准） ==========

    /// 截图并缩放到 720p 基准分辨率，供识别使用（模板 / OCR / 颜色判断均用此图）。
    pub fn screencap_for_recognition(&mut self) -> Result<RgbaImage> {
        self.check_stop()?;
        let raw = self.screencap.screencap()?;
        Ok(self.resolution.scale_screenshot_to_base(&raw))
    }

    // ========== 输入（统一 720p → 实际分辨率缩放） ==========

    /// 点击 720p 基准坐标点（自动缩放），点击后鼠标回到窗口中心，
    /// 避免按钮 hover 变化干扰后续识别。
    pub fn click_at_720p(&mut self, x: u32, y: u32) -> Result<()> {
        self.check_stop()?;
        let (sx, sy) = self.resolution.scale_point(x, y);
        self.input.click(
            Contact::Left,
            Point2D {
                x: sx as i32,
                y: sy as i32,
            },
        )?;
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()?;
        Ok(())
    }

    /// 将鼠标移动到安全位置（窗口中心），避免 hover 干扰识别。
    fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        let cx = self.resolution.width as i32 / 2;
        let cy = self.resolution.height as i32 / 2;
        self.input
            .touch_move(Contact::Left, Point2D { x: cx, y: cy })
    }

    /// 按下并松开键盘按键（虚拟键码），如 ESC=0x1B。
    pub fn press_key(&mut self, vk_code: i32) -> Result<()> {
        self.check_stop()?;
        self.input.press_key(vk_code)
    }

    // ========== 模板匹配 ==========

    /// 在 720p 截图的指定 ROI 内搜索模板。
    ///
    /// 模板名需带子目录前缀（如 `"情报档案库/下一篇.png"`）。
    /// 返回 `Ok(Some(MatchResult))` 命中、`Ok(None)` 未命中 / 分数过低 / 模板缺失。
    pub fn find_template_in_roi(
        &mut self,
        screenshot: &RgbaImage,
        template_name: &str,
        roi: Region2D<u32>,
        threshold: f32,
    ) -> Result<Option<MatchResult>> {
        // 直接传 RgbaImage 引用：泛型接口内部灰度化，无需克隆 / 颜色转换。
        let result = self
            .templates
            .match_template_in_region(screenshot, template_name, Some(roi));
        match result {
            Ok(m) if m.score >= threshold => Ok(Some(m)),
            Ok(_) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// 在 720p 截图指定 ROI 内搜索模板，找到后点击其中心（自动缩放）。
    pub fn find_and_click_template(
        &mut self,
        screenshot: &RgbaImage,
        template_name: &str,
        roi: Region2D<u32>,
        threshold: f32,
    ) -> Result<bool> {
        if let Some(m) = self.find_template_in_roi(screenshot, template_name, roi, threshold)? {
            let center = m.region.center();
            self.click_at_720p(center.x, center.y)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // ========== OCR ==========

    /// 对 720p 截图 ROI 区域做 OCR，返回识别文本（自动裁剪 + 单行检测）。
    pub fn ocr_in_roi(&mut self, screenshot: &RgbaImage, roi: Region2D<u32>) -> Result<String> {
        let cropped = imageops::crop_imm(screenshot, roi.x0(), roi.y0(), roi.width(), roi.height())
            .to_image();
        let rgb = DynamicImage::ImageRgba8(cropped).to_rgb8();
        if let Some(region) =
            text_detection::detect_single_line(&rgb, 128, ThresholdType::Binary, 6)
        {
            let cropped = imageops::crop_imm(
                &rgb,
                region.x0(),
                region.y0(),
                region.width(),
                region.height(),
            )
            .to_image();
            let output = self.ocr.lock().unwrap().ocr(&cropped)?;
            let text = output
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
            Ok(text)
        } else {
            Ok("".into())
        }
    }

    // ========== 颜色判断 ==========

    /// 判断 720p 截图 ROI（ltwh）区域平均灰度是否低于阈值（即深色 / 选中态）。
    pub fn is_roi_dark_ltwh(
        &self,
        screenshot: &RgbaImage,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        threshold: u8,
    ) -> bool {
        self.is_roi_dark(screenshot, Region2D::from_ltwh(x, y, w, h), threshold)
    }

    /// 判断 720p 截图 ROI 区域平均灰度是否低于阈值。
    fn is_roi_dark(&self, screenshot: &RgbaImage, roi: Region2D<u32>, threshold: u8) -> bool {
        let cropped = imageops::crop_imm(screenshot, roi.x0(), roi.y0(), roi.width(), roi.height());
        let gray = imageops::grayscale(&cropped.to_image());
        let total: u64 = gray.pixels().map(|p| p.0[0] as u64).sum();
        let pixel_count = (roi.width() * roi.height()) as u64;
        if pixel_count == 0 {
            return false;
        }
        ((total / pixel_count) as u8) < threshold
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
