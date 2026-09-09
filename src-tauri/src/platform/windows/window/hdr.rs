//! HDR 检测：判断指定窗口所在显示器是否开启了 HDR（高动态范围）。
//!
//! 原理：`MonitorFromWindow` 定位窗口所在显示器 → `CreateDXGIFactory1` 枚举
//! DXGI 输出并按 `HMONITOR` 匹配 → `IDXGIOutput6::GetDesc1` 读取当前色彩空间。
//! 只有 `DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020` 表示当前处于 HDR 模式。

use ::windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020;
use ::windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIFactory1, IDXGIOutput6,
};
use ::windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use ::windows::core::Interface;
use anyhow::{Result, bail};

use crate::platform::WindowHandle;

/// 判断指定窗口所在显示器是否开启了 HDR。
///
/// 检测失败（API 不可用 / 查询出错）时返回 `Err`，由调用方决定是否阻断任务。
pub(in crate::platform) fn is_hdr_enabled_on_window_monitor(window: WindowHandle) -> Result<bool> {
    let hwnd = window.raw;
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_invalid() {
        bail!("无法获取窗口所在显示器");
    }

    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    let mut adapter_index = 0;
    loop {
        let adapter = match unsafe { factory.EnumAdapters1(adapter_index) } {
            Ok(adapter) => adapter,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => return Err(error.into()),
        };
        let mut output_index = 0;
        loop {
            let output = match unsafe { adapter.EnumOutputs(output_index) } {
                Ok(output) => output,
                Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(error) => return Err(error.into()),
            };
            let output6: IDXGIOutput6 = output.cast()?;
            let description = unsafe { output6.GetDesc1()? };
            if description.Monitor == monitor {
                return Ok(description.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020);
            }
            output_index += 1;
        }
        adapter_index += 1;
    }

    bail!("未找到窗口所在显示器对应的 DXGI 输出")
}
