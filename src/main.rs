#![allow(unused)]

use std::{thread, time::Duration};

use anyhow::{Result, anyhow, bail};
use image::{DynamicImage, RgbImage, RgbaImage, imageops};
use rapidocr_core::{config::PipelineConfig, types::OcrOutput};

use dak::hotkey::HotkeyListener;
use dak::input::{Contact, InputBase, SeizeInput};
use dak::ocr;
use dak::ocr::text_detection;
use dak::screencap::PrintWindowScreencap;
use dak::template_matching;
use dak::utils::point::{Point2D, Region2D};
use dak::utils::timeit::timeit_print;

const THRESHOLD: u8 = 128;
const PADDING: u32 = 6;

fn main() -> Result<()> {
    dak::set_thread_dpi_awareness_context();

    let hwnd = dak::window::get_window_by_title("Endfield", Some("UnityWndClass"))?;
    dak::window::ensure_foreground_and_topmost(hwnd)?;
    dak::window::ensure_window_on_screen(hwnd)?;
    let client_rect = dak::window::get_client_rect(hwnd)?;
    if client_rect.width() != 1280 || client_rect.height() != 720 {
        bail!("Window size is not 1280×720");
    }

    let mut screencap = PrintWindowScreencap::new(hwnd);
    let mut input = SeizeInput::new(hwnd, false);
    let pipeline_config = PipelineConfig::recognition_only();

    let next_button_template = template_matching::load_template("resources/templates/下一篇.png")?;
    let next_button_region = Region2D::from_ltrb(762, 654, 925, 711);

    let hotkey = HotkeyListener::alt_delete();
    println!("按 Alt+Delete 停止");

    loop {
        if hotkey.stop_requested() {
            println!("收到停止信号，退出");
            break;
        }
        let image = screencap.screencap()?;
        let image = DynamicImage::ImageRgba8(image).to_rgb8();
        let ocr_roi = imageops::crop_imm(&image, 350, 58, 578, 42).to_image();

        if let Some(region) = text_detection::detect_single_line(&ocr_roi, THRESHOLD, PADDING) {
            let text_image = text_detection::crop_region(&ocr_roi, region);
            let ocr_output = ocr::ocr(&text_image, pipeline_config)?;
            println!(
                "{}",
                ocr_output
                    .lines
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n")
            );
        } else {
            println!("未检测到文字区域");
        }

        // 模板匹配：在指定区域查找 “下一篇” 按钮
        if let Some(m) = template_matching::match_template_in_region(
            &image,
            next_button_region,
            &next_button_template,
        ) {
            println!("模板匹配分数: {:.3}", m.score);
            if m.score > 0.75 {
                let Point2D {
                    x: click_x,
                    y: click_y,
                } = next_button_region.center();
                input.click(Contact::Left, click_x as i32, click_y as i32)?;
            } else {
                input.click(Contact::Left, 1244, 360)?;
            }
        }

        thread::sleep(Duration::from_secs_f64(0.4));
    }

    Ok(())
}
