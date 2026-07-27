#![allow(unused)]

use std::{thread, time::Duration};

use anyhow::{Result, anyhow, bail};
use image::{DynamicImage, RgbImage, RgbaImage, imageops};
use rapidocr_core::config::PipelineConfig;

use dak::input::{Contact, InputBase, SeizeInput};
use dak::ocr;
use dak::screencap::PrintWindowScreencap;
use dak::utils::Rect;

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

    loop {
        let image: RgbaImage = screencap.screencap()?;
        let image: RgbImage = DynamicImage::ImageRgba8(image).to_rgb8();
        let cropped: RgbImage = imageops::crop_imm(&image, 378, 59, 203, 41).to_image();
        let ocr_output = ocr::ocr(&cropped, pipeline_config)?;
        println!("OCR output: {:?}", ocr_output);

        input.click(Contact::Left, 1244, 360)?;
        thread::sleep(Duration::from_secs_f64(0.6));
        break;
    }

    Ok(())
}
