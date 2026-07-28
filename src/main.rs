use std::{thread, time::Duration};

use anyhow::{Result, bail};
use dak::{
    hotkey::HotkeyListener,
    input::{Contact, InputBase, SeizeInput},
    ocr::{OcrEngine, text_detection},
    screencap::PrintWindowScreencap,
    template_matching::TemplateManager,
    utils::region::Region2D,
};
use image::{DynamicImage, imageops};
use imageproc::contrast::ThresholdType;
use rapidocr_core::config::PipelineConfig;
use tracing::{debug, info, trace};

fn main() -> Result<()> {
    let _logger_guard = dak::logger::init();

    dak::set_thread_dpi_awareness_context();

    let hwnd = dak::window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
    dak::window::ensure_foreground_and_topmost(hwnd)?;
    dak::window::ensure_window_on_screen(hwnd)?;
    let client_rect = dak::window::get_client_rect(hwnd)?;
    if client_rect.width() != 1280 || client_rect.height() != 720 {
        bail!("Window size is not 1280×720");
    }

    let mut template_manager = TemplateManager::new("resources/templates");
    let mut screencap = PrintWindowScreencap::new(hwnd);
    let mut input = SeizeInput::new(hwnd, false);
    let pipeline_config = PipelineConfig::recognition_only();
    let mut ocr_engine = OcrEngine::new(pipeline_config)?;

    let 下一篇模板名称 = "下一篇.png";
    let 档案库右箭头模板名称 = "档案库右箭头.png";
    let next_button_region = Region2D::from_ltrb(762, 654, 925, 711);
    let arrow_right_region = Region2D::from_ltrb(1206, 313, 1276, 423);

    // 注册快捷键
    let hotkey = HotkeyListener::alt_delete();
    info!("按 Alt+Delete 停止");

    // 导航到档案库页面
    // ...

    loop {
        if hotkey.stop_requested() {
            info!("收到停止信号，退出");
            break;
        }

        let image = screencap.screencap()?;
        let image = DynamicImage::ImageRgba8(image).to_rgb8();
        let ocr_roi = imageops::crop_imm(&image, 350, 58, 578, 42).to_image();

        if let Some(region) =
            text_detection::detect_single_line(&ocr_roi, 128, ThresholdType::Binary, 6)
        {
            let text_image = text_detection::crop_region(&ocr_roi, region);
            let ocr_output = ocr_engine.ocr(&text_image)?;
            info!(
                "{}",
                ocr_output
                    .lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n")
            );
        } else {
            debug!("未检测到文字区域");
        }

        // 查找 “下一篇” 按钮
        if let Ok(m) = template_manager.match_template_in_region(
            &image,
            下一篇模板名称,
            Some(next_button_region),
        ) {
            trace!("模板匹配分数：{:.4}", m.score);
            if m.score > 0.75 {
                input.click(Contact::Left, next_button_region.center().cast())?;
            } else if let Ok(m) = template_manager.match_template_in_region(
                &image,
                档案库右箭头模板名称,
                Some(arrow_right_region),
            ) {
                trace!("模板匹配分数：{:.4}", m.score);
                if m.score > 0.75 {
                    input.click(Contact::Left, arrow_right_region.center().cast())?;
                } else {
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
        // 鼠标回中，避免 hover 到按钮上导致按钮变化
        input.touch_move(Contact::Left, client_rect.center())?;

        thread::sleep(Duration::from_millis(300));
    }

    Ok(())
}
