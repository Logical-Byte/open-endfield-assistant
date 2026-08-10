//! HDR 检测：判断指定窗口所在显示器是否开启了 HDR（高动态范围）。
//!
//! 原理：`MonitorFromWindow` 定位窗口所在显示器 → `GetMonitorInfoW` 读取其
//! 桌面矩形 → `QueryDisplayConfig` 枚举活动显示路径并用几何位置匹配到对应
//! target → `DisplayConfigGetDeviceInfo(ADVANCED_COLOR_INFO)` 读取
//! `advancedColorEnabled`（Value 位 1）判断 HDR 是否开启。
//!
//! 参考实现：MaaEnd `agent/go-service/taskersink/hdrcheck/hdr_windows.go`。

use std::mem::size_of;

use anyhow::{Result, bail};
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_MODE, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HWND};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW, MonitorFromWindow,
};

/// 判断指定窗口所在显示器是否开启了 HDR。
///
/// 检测失败（API 不可用 / 查询出错）时返回 `Err`，由调用方决定是否阻断任务。
pub fn is_hdr_enabled_on_window_monitor(hwnd: HWND) -> Result<bool> {
    // 1. 定位窗口所在显示器，读取其桌面矩形
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_invalid() {
        bail!("无法获取窗口所在显示器");
    }

    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    unsafe { GetMonitorInfoW(monitor, &mut monitor_info.monitorInfo) }.ok()?;

    // 2. 枚举活动显示路径，按几何位置匹配窗口所在显示器
    let (paths, mode_infos) = query_active_paths()?;
    for path in &paths {
        let Some(source) = source_mode_of(path, &mode_infos) else {
            continue;
        };

        let rc = monitor_info.monitorInfo.rcMonitor;
        let position_matches = source.position.x == rc.left && source.position.y == rc.top;
        let size_matches =
            source.width as i32 == rc.right - rc.left && source.height as i32 == rc.bottom - rc.top;
        if !(position_matches && size_matches) {
            continue;
        }

        return advanced_color_enabled(path);
    }

    bail!("未找到窗口所在显示器对应的活动显示路径");
}

/// 查询活动显示路径（`QDC_ONLY_ACTIVE_PATHS`），带 `ERROR_INSUFFICIENT_BUFFER` 重试。
fn query_active_paths() -> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>)> {
    let mut num_paths = 0u32;
    let mut num_modes = 0u32;

    let mut result = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
    };
    if result != ERROR_SUCCESS {
        bail!("GetDisplayConfigBufferSizes 失败: {}", result.0);
    }
    if num_paths == 0 {
        return Ok((Vec::new(), Vec::new()));
    }

    loop {
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); num_paths as usize];
        let mut mode_infos = vec![DISPLAYCONFIG_MODE_INFO::default(); num_modes as usize];

        result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut num_paths,
                paths.as_mut_ptr(),
                &mut num_modes,
                mode_infos.as_mut_ptr(),
                None,
            )
        };
        if result == ERROR_SUCCESS {
            paths.truncate(num_paths as usize);
            mode_infos.truncate(num_modes as usize);
            return Ok((paths, mode_infos));
        }
        if result != ERROR_INSUFFICIENT_BUFFER {
            bail!("QueryDisplayConfig 失败: {}", result.0);
        }
        // 缓冲区不足（显示拓扑变化），按返回的新大小重试
    }
}

/// 取路径对应的 source mode（显示器在虚拟桌面中的位置与尺寸）。
fn source_mode_of(
    path: &DISPLAYCONFIG_PATH_INFO,
    mode_infos: &[DISPLAYCONFIG_MODE_INFO],
) -> Option<DISPLAYCONFIG_SOURCE_MODE> {
    let index = unsafe { path.sourceInfo.Anonymous.modeInfoIdx } as usize;
    let mode = mode_infos.get(index)?;
    if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return None;
    }
    Some(unsafe { mode.Anonymous.sourceMode })
}

/// 查询指定 target 是否开启高级颜色（HDR）。
///
/// Value 位标志：位 0 = advancedColorSupported，位 1 = advancedColorEnabled（HDR 开启），
/// 位 2 = wideColorEnforced，位 3 = advancedColorForceDisabled。
fn advanced_color_enabled(path: &DISPLAYCONFIG_PATH_INFO) -> Result<bool> {
    let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
    info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
    info.header.size = size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
    info.header.adapterId = path.targetInfo.adapterId;
    info.header.id = path.targetInfo.id;

    let result = unsafe { DisplayConfigGetDeviceInfo(&mut info.header) };
    if result != 0 {
        bail!("DisplayConfigGetDeviceInfo 失败: {result}");
    }

    Ok(unsafe { info.Anonymous.value } & 0x2 != 0)
}
