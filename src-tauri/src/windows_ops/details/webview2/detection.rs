//! WebView2 Runtime 安装状态检测（注册表）。

use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use crate::windows_ops::registry::read_registry_string;

/// 注册表检测位置（与 Tauri 安装器 `main.wxs` 的 `RegistrySearch` 一致）：
/// - HKLM：per-machine 安装（64 位系统走 WOW6432Node 视图）
/// - HKCU：per-user 安装
///
/// <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution?tabs=dotnetcsharp#detect-if-a-webview2-runtime-is-already-installed>
const REGISTRY_LOCATIONS: &[(HKEY, &str, &str)] = &[
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        "pv",
    ),
    (
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        "pv",
    ),
    (
        HKEY_CURRENT_USER,
        r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        "pv",
    ),
];

/// 检测系统是否已安装 Microsoft Edge WebView2。
///
/// 依据微软官方文档读取注册表 `pv` 值：任一位置存在、非空且非 `0.0.0.0` 即视为已安装。
///
/// <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution?tabs=dotnetcsharp#detect-if-a-webview2-runtime-is-already-installed>
pub fn is_installed() -> bool {
    for (root, subkey, value_name) in REGISTRY_LOCATIONS {
        match read_registry_string(*root, subkey, value_name) {
            Ok(Some(pv)) if !pv.is_empty() && pv != "0.0.0.0" => {
                // 任一位置存在、非空且非 `0.0.0.0`，返回 `true`
                return true;
            }
            _ => {} // 其他情况（值为空、值为 `0.0.0.0`、键或值不存在、读取注册表失败等）都继续尝试下一个位置
        }
    }
    false
}
