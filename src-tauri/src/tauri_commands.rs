//! Tauri 命令层：薄胶水，把前端 `invoke` 转发给 [`crate::controller::Controller`]。

use std::{fs, sync::Arc};

use anyhow::Result;
use tauri::State;

use crate::{
    app_paths::AppPaths,
    controller::{AppStatus, Controller},
    types::PrtsData,
};

/// 启动扫描档案库任务（在后台线程执行，立即返回当前状态）。
#[tauri::command]
pub fn start_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().start_scan();
    state.inner().get_status()
}

/// 请求停止扫描档案库任务。
#[tauri::command]
pub fn stop_scan(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().stop_scan();
    state.inner().get_status()
}

/// 查询当前应用状态。
#[tauri::command]
pub fn get_status(state: State<'_, Arc<Controller>>) -> AppStatus {
    state.inner().get_status()
}

/// 返回 prts.json 完整数据（前端用于分类中文名映射与自动补全候选）。
#[tauri::command]
pub fn get_prts_data(state: State<'_, Arc<Controller>>) -> Arc<PrtsData> {
    state.inner().prts_data()
}

/// 退出程序。
#[tauri::command]
pub fn quit(state: State<'_, Arc<Controller>>) {
    state.inner().quit();
}

/// 在系统文件管理器中打开日志目录（不存在时先创建）。
///
/// 由于根目录为双模式动态路径（dev=项目根 / release=exe 目录），静态 scope 无法精确
/// 表达，故不用前端 `openPath` + scope 方案，而用后端 Rust API `open_path`
/// （直接调不经 scope 检查），capabilities 无需放通任何路径。
#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let logs_dir = AppPaths::new()
        .map_err(|e| format!("无法定位日志目录: {e}"))?
        .logs_dir();
    fs::create_dir_all(&logs_dir).map_err(|e| format!("无法创建日志目录: {e}"))?;
    tauri_plugin_opener::open_path(&logs_dir, None::<&str>)
        .map_err(|e| format!("无法打开日志目录: {e}"))
}

/// 设置"关闭窗口时最小化到托盘"（返回设置后的值，供前端设置界面使用）。
#[tauri::command]
pub fn set_minimize_to_tray(enabled: bool) -> bool {
    crate::tray::set_minimize_to_tray(enabled);
    crate::tray::get_minimize_to_tray()
}

/// 查询"关闭窗口时最小化到托盘"。
#[tauri::command]
pub fn get_minimize_to_tray() -> bool {
    crate::tray::get_minimize_to_tray()
}

/// 截取游戏窗口画面：按指定尺寸缩放并编码为指定格式（png / jpeg / webp），
/// 返回 base64 编码的图片数据（不含 data URL 前缀，由前端拼接）。
///
/// 本命令每次调用只执行一次截图；帧率控制、定时轮询等逻辑全部由前端负责。
#[tauri::command]
pub fn screenshot(width: u32, height: u32, format: String) -> Result<String, String> {
    use base64::Engine as _;
    use image::{ImageFormat, imageops};
    use std::io::Cursor;

    // 1. 定位游戏窗口（PrintWindow 可捕获非最小化后台窗口）
    let hwnd = crate::window::get_window_by_title(
        crate::window::ENDFIELD_WINDOW_TITLE,
        Some(crate::window::ENDFIELD_WINDOW_CLASS),
    )
    .map_err(|e| format!("未找到游戏窗口: {e}"))?;

    // 2. 截图
    let mut screencap = crate::screencap::PrintWindowScreencap::new(hwnd);
    let raw = screencap
        .screencap()
        .map_err(|e| format!("截图失败: {e}"))?;

    // 3. 缩放到指定尺寸
    let resized = imageops::resize(
        &raw,
        width.max(1),
        height.max(1),
        imageops::FilterType::Triangle,
    );

    // 4. 按格式编码（JPEG 不支持 alpha 通道，先转 RGB 再编码）
    let image_format = match format.as_str() {
        "png" => ImageFormat::Png,
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "webp" => ImageFormat::WebP,
        other => {
            return Err(format!(
                "不支持的图片格式: {other}（可选 png / jpeg / webp）"
            ));
        }
    };
    let mut buf = Cursor::new(Vec::new());
    if image_format == ImageFormat::Jpeg {
        image::DynamicImage::ImageRgba8(resized)
            .to_rgb8()
            .write_to(&mut buf, image_format)
            .map_err(|e| format!("图片编码失败: {e}"))?;
    } else {
        resized
            .write_to(&mut buf, image_format)
            .map_err(|e| format!("图片编码失败: {e}"))?;
    }

    // 5. base64 编码返回
    Ok(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}
