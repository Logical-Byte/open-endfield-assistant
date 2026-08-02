//! 脚本会话模块。
//!
//! `Session` 是贯穿整个任务生命周期的统一上下文，聚合了截图、输入、OCR、
//! 模板匹配、分辨率等所有底层能力。类似于 MaaFramework 中的 Tasker。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// 模板资源根目录
const TEMPLATES_ROOT: &str = "resources/templates";

/// 脚本会话，聚合所有底层能力。
///
/// 所有识别操作在 720p 基准分辨率上进行（截图自动缩放），
/// 所有点击操作自动将坐标从 720p 缩放到实际分辨率。
pub struct Session {
    /// 游戏窗口句柄
    pub hwnd: HWND,
    /// 截图器
    screencap: Box<dyn ScreencapBase>,
    /// 输入器
    input: Box<dyn InputBase>,
    /// OCR 引擎
    ocr: OcrEngine,
    /// 模板匹配管理器，管理所有模板图片（含子目录），按需懒加载并缓存。
    templates: TemplateManager,
    /// 游戏实际分辨率
    pub resolution: GameResolution,
    /// 停止标志（来自热键监听器），每次操作前检查
    stop_flag: Arc<AtomicBool>,
}

impl Session {
    /// 创建新的 Session。
    ///
    /// # 参数
    /// - `hwnd`: 游戏窗口句柄
    /// - `screencap`: 截图器实例
    /// - `input`: 输入器实例
    /// - `ocr`: OCR 引擎实例
    /// - `resolution`: 游戏实际分辨率
    /// - `stop_flag`: 热键停止标志，每次操作前检查
    pub fn new(
        hwnd: HWND,
        screencap: Box<dyn ScreencapBase>,
        input: Box<dyn InputBase>,
        ocr: OcrEngine,
        resolution: GameResolution,
        stop_flag: Arc<AtomicBool>,
    ) -> Self {
        // 从根目录加载所有模板（含情报档案库子目录）
        Self {
            hwnd,
            screencap,
            input,
            ocr,
            templates: TemplateManager::new(TEMPLATES_ROOT),
            resolution,
            stop_flag,
        }
    }

    /// 检查是否收到停止信号，如果收到则返回 [`TaskStopped`] 错误中断执行。
    ///
    /// 停止不是"任务出错"，上层应将其与真正的错误区分开（见 [`crate::task::TaskStopped`]）。
    fn check_stop(&self) -> Result<()> {
        if self.stop_flag.load(Ordering::Relaxed) {
            Err(TaskStopped.into())
        } else {
            Ok(())
        }
    }

    /// 清除停止标志（启动新任务或单次扫描前调用，避免上一次的停止信号残留）。
    pub fn reset_stop(&mut self) {
        self.stop_flag.store(false, Ordering::Relaxed);
    }

    // ========== 截图相关 ==========

    /// 截图并缩放到 720p 基准分辨率，用于识别。
    ///
    /// 所有模板匹配、OCR、颜色判断都应使用此方法获取截图。
    pub fn screencap_for_recognition(&mut self) -> Result<RgbaImage> {
        self.check_stop()?;
        let raw = self.screencap.screencap()?;
        Ok(self.resolution.scale_screenshot_to_base(&raw))
    }

    /// 截图保持原始分辨率（用于调试/保存）。
    #[allow(dead_code)]
    pub fn screencap_raw(&mut self) -> Result<RgbaImage> {
        self.screencap.screencap()
    }

    // ========== 输入相关 ==========

    /// 点击 720p 基准坐标系中的点（自动缩放到实际分辨率）。
    /// 点击完成后自动将鼠标移回窗口中心，避免 hover 效果干扰后续识别。
    pub fn click_at_720p(&mut self, x: u32, y: u32) -> Result<()> {
        self.check_stop()?;
        let (sx, sy) = self.resolution.scale_point(x, y);
        let point = Point2D {
            x: sx as i32,
            y: sy as i32,
        };
        self.input.click(Contact::Left, point)?;
        // 点击后回中，避免按钮 hover 变化
        thread::sleep(Duration::from_millis(50));
        self.move_mouse_to_safe_position()?;
        Ok(())
    }

    /// 在 720p 基准矩形区域内随机取一点点击（越靠近中心概率越高）。
    /// 主要用于点击 ROI 匹配到的模板中心。
    #[allow(dead_code)]
    pub fn click_in_720p_region(
        &mut self,
        left: u32,
        top: u32,
        right: u32,
        bottom: u32,
    ) -> Result<()> {
        // 取区域中心点击
        let cx = (left + right) / 2;
        let cy = (top + bottom) / 2;
        self.click_at_720p(cx, cy)
    }

    /// 将鼠标移动到安全位置（窗口中心），避免 hover 效果干扰识别。
    pub fn move_mouse_to_safe_position(&mut self) -> Result<()> {
        let cx = self.resolution.width as i32 / 2;
        let cy = self.resolution.height as i32 / 2;
        let point = Point2D { x: cx, y: cy };
        self.input.touch_move(Contact::Left, point)
    }

    /// 按下键盘按键（虚拟键码），委托给 InputBase 实现。
    /// 例如 ESC 键为 0x1B。
    pub fn press_key(&mut self, vk_code: i32) -> Result<()> {
        self.check_stop()?;
        self.input.press_key(vk_code)
    }

    // ========== 模板匹配相关 ==========

    /// 在 720p 基准截图的指定 ROI 内搜索模板。
    ///
    /// # 参数
    /// - `screenshot`: 已缩放到 720p 的截图（由 `screencap_for_recognition` 获取）
    /// - `template_name`: 模板文件名。如果在 `情报档案库/` 子目录中，需要带 `情报档案库/` 前缀
    /// - `roi`: 720p 基准的搜索区域
    /// - `threshold`: 匹配阈值（0.0 ~ 1.0），低于此分数视为未匹配
    ///
    /// # 返回
    /// - `Ok(Some(MatchResult))`: 匹配成功
    /// - `Ok(None)`: 未匹配（低于阈值或模板不存在）
    pub fn find_template_in_roi(
        &mut self,
        screenshot: &RgbaImage,
        template_name: &str,
        roi: Region2D<u32>,
        threshold: f32,
    ) -> Result<Option<MatchResult>> {
        // 将 RgbaImage 转换为 RgbImage 用于模板匹配
        let rgb_screenshot = DynamicImage::ImageRgba8(screenshot.clone()).to_rgb8();
        let result =
            self.templates
                .match_template_in_region(&rgb_screenshot, template_name, Some(roi));

        match result {
            Ok(m) if m.score >= threshold => Ok(Some(m)),
            Ok(_) => Ok(None),  // 匹配到了但分数太低
            Err(_) => Ok(None), // 模板文件不存在等情况
        }
    }

    /// 在 720p 截图指定 ROI 内搜索模板，找到后点击其中心（720p 基准坐标，自动缩放）。
    ///
    /// 返回 `true` 表示找到并点击成功，`false` 表示未找到。
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

    // ========== OCR 相关 ==========

    /// 在 720p 截图的指定 ROI 内进行 OCR 识别，返回识别到的文本。
    ///
    /// 自动将 ROI 裁剪出来并转为 RGB 格式进行识别。
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
            let output = self.ocr.ocr(&cropped)?;
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

    // ========== 颜色判断相关 ==========

    /// 判断 720p 截图中指定 ROI 区域的平均灰度值是否低于阈值（即为深色）。
    ///
    /// 用于判断档案库子界面的侧边栏 tab 是否处于选中状态。
    /// 注意：screenshot 需要是 RgbImage 格式（由 `screencap_for_recognition` 返回的 RgbaImage 转换而来）。
    pub fn is_roi_dark(&self, screenshot: &RgbaImage, roi: Region2D<u32>, threshold: u8) -> bool {
        let cropped = imageops::crop_imm(screenshot, roi.x0(), roi.y0(), roi.width(), roi.height());
        let gray = imageops::grayscale(&cropped.to_image());
        let total: u64 = gray.pixels().map(|p| p.0[0] as u64).sum();
        let pixel_count = (roi.width() * roi.height()) as u64;
        if pixel_count == 0 {
            return false;
        }
        let avg = (total / pixel_count) as u8;
        avg < threshold
    }

    /// 判断 720p 截图中指定 ltwh ROI 区域是否为深色。
    #[allow(dead_code)]
    pub fn is_roi_dark_ltwh(
        &self,
        screenshot: &RgbaImage,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        threshold: u8,
    ) -> bool {
        let roi = Region2D::from_ltwh(x, y, w, h);
        self.is_roi_dark(screenshot, roi, threshold)
    }

    // ========== 截图器/输入器切换 ==========

    /// 替换截图器（运行时切换）。
    pub fn set_screencap(&mut self, screencap: Box<dyn ScreencapBase>) {
        self.screencap = screencap;
    }

    /// 替换输入器（运行时切换）。
    #[allow(dead_code)]
    pub fn set_input(&mut self, input: Box<dyn InputBase>) {
        self.input = input;
    }
}
