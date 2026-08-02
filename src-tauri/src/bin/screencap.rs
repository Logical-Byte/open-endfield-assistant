#![allow(unused)]

use anyhow::Result;
use app_lib::{
    screencap::{
        DesktopDupScreencap, DesktopDupWindowScreencap, FramePoolScreencap, GdiScreencap,
        PrintWindowScreencap, ScreenDCScreencap, ScreencapBase,
    },
    utils::{region::Region2D, timeit::timeit_print},
    window::{
        self, ensure_foreground_and_topmost, ensure_window_on_screen, get_active_window,
        get_client_rect, get_window_by_title, get_window_class_name, get_window_title,
    },
};

/// 对一种截屏方式执行全屏 + 区域截屏，计时并保存到 temp/
fn test_screencap(
    screencap: &mut impl ScreencapBase,
    label: &str,
    file_prefix: &str,
    region: Region2D<i32>,
) -> Result<()> {
    // 先预热
    screencap.screencap()?;
    screencap.screencap_region(region)?;

    let image = timeit_print(|| screencap.screencap(), &format!("{label} Screencap"))?;
    image.save(format!("temp/{file_prefix}_screencap.png"))?;

    let image = timeit_print(
        || screencap.screencap_region(region),
        &format!("{label} Screencap region"),
    )?;
    image.save(format!("temp/{file_prefix}_screencap_region.png"))?;

    Ok(())
}

fn main() -> Result<()> {
    window::set_thread_dpi_awareness_context();

    let hwnd = get_window_by_title("Endfield", Some("UnityWndClass"))?;
    // let hwnd = get_active_window();
    let title = get_window_title(hwnd)?;
    let class_name = get_window_class_name(hwnd)?;
    let client_rect = get_client_rect(hwnd)?;
    dbg!(hwnd, title, class_name, client_rect);

    ensure_foreground_and_topmost(hwnd)?;
    ensure_window_on_screen(hwnd)?;

    let mut print_window_screencap = PrintWindowScreencap::new(hwnd);
    let mut gdi_screencap = GdiScreencap::new(hwnd);
    let mut screen_dc_screencap = ScreenDCScreencap::new(hwnd);
    let mut frame_pool_screencap = FramePoolScreencap::new(hwnd);
    let mut desktop_dup_screencap = DesktopDupScreencap::new(hwnd);
    let mut desktop_dup_window_screencap = DesktopDupWindowScreencap::new(hwnd);

    let region = Region2D::from_ltwh(100, 100, 300, 100);

    test_screencap(
        &mut print_window_screencap,
        "PrintWindow",
        "print_window",
        region,
    )?;
    test_screencap(&mut gdi_screencap, "GDI", "gdi", region)?;
    test_screencap(&mut screen_dc_screencap, "ScreenDC", "screen_dc", region)?;
    test_screencap(&mut frame_pool_screencap, "FramePool", "frame_pool", region)?;
    test_screencap(
        &mut desktop_dup_screencap,
        "DesktopDup",
        "desktop_dup",
        region,
    )?;
    // Desktop Duplication 同一输出只能有一个实例，先释放前一个
    drop(desktop_dup_screencap);
    test_screencap(
        &mut desktop_dup_window_screencap,
        "DesktopDupWindow",
        "desktop_dup_window",
        region,
    )?;

    Ok(())
}
