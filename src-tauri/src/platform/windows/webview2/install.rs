//! Microsoft Edge WebView2 自动安装（在线下载 Evergreen Bootstrapper）。
//!
//! 微软官方 “在线部署” 方案：
//! 1. 注册表检测确认未安装（见 [`super::detection`]）；
//! 2. 用 `reqwest` 从官方链接下载 Evergreen Bootstrapper（约 2 MB）；
//! 3. 运行 Evergreen Bootstrapper
//! 4. 等待安装程序退出（退出码 0），重跑注册表检测确认。
//!
//! 参考：<https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution>

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tracing::{debug, error, info, warn};

use crate::{
    locale::UiLocale,
    platform::dialog::{self, DialogIcon},
};

/// Evergreen Bootstrapper 官方下载链接（微软固定跳转链接，约 2 MB，自动匹配架构）。
const BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/// 下载超时（下载本体约 2 MB，正常情况下很快）。
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// 确保 Microsoft Edge WebView2 可用。
///
/// `cache_dir`：应用缓存目录（项目根目录内），用于暂存下载的引导程序，
///
/// 已安装 → 直接返回 `true`；未安装 → 弹窗征询用户，同意后下载引导程序并安装。
/// 安装成功 → 返回 `true`；安装失败或用户拒绝 → 返回 `false`。
pub(in crate::platform) fn ensure_installed(
    cache_dir: &Path,
    locale: UiLocale,
) -> anyhow::Result<bool> {
    if super::detection::is_installed() {
        debug!("Microsoft Edge WebView2 已安装");
        return Ok(true);
    }

    info!("未检测到 Microsoft Edge WebView2，准备自动安装");
    if !confirm_install(locale) {
        error!("用户拒绝安装 Microsoft Edge WebView2");
        return Ok(false);
    }

    match install(cache_dir) {
        Ok(()) if super::detection::is_installed() => {
            info!("Microsoft Edge WebView2 安装成功");
            Ok(true)
        }
        Ok(()) => {
            warn!("WebView2 安装程序已结束，但重检仍未检测到 Runtime");
            dialog::show_message(
                failure_title(locale),
                &failure_message(locale, None),
                DialogIcon::Error,
            )?;
            Ok(false)
        }
        Err(e) => {
            warn!("WebView2 自动安装失败: {e}");
            dialog::show_message(
                failure_title(locale),
                &failure_message(locale, Some(&e)),
                DialogIcon::Error,
            )?;
            Ok(false)
        }
    }
}

/// 征询用户是否立即联网安装。
fn confirm_install(locale: UiLocale) -> bool {
    let (title, content) = match locale {
        UiLocale::ZhCn => (
            "OEA 需要 WebView2",
            "检测到系统未安装 Microsoft Edge WebView2（Windows 的网页渲染组件），OEA 的界面将无法显示。\n\n是否立即联网下载并安装？\n也可以选择“否”，稍后前往 <a href=\"https://developer.microsoft.com/microsoft-edge/webview2/consumer/\">https://developer.microsoft.com/microsoft-edge/webview2/consumer/</a> 手动安装。",
        ),
        UiLocale::EnUs => (
            "OEA requires WebView2",
            "Microsoft Edge WebView2, the Windows web rendering component, is not installed. OEA cannot display its interface without it.\n\nDownload and install it now?\nChoose No to install it later from <a href=\"https://developer.microsoft.com/microsoft-edge/webview2/consumer/\">https://developer.microsoft.com/microsoft-edge/webview2/consumer/</a>.",
        ),
    };
    dialog::confirm(title, content, DialogIcon::Info).unwrap_or(false)
}

fn failure_title(locale: UiLocale) -> &'static str {
    match locale {
        UiLocale::ZhCn => "WebView2 安装失败",
        UiLocale::EnUs => "WebView2 installation failed",
    }
}

fn failure_message(locale: UiLocale, error: Option<&anyhow::Error>) -> String {
    let reason = match (locale, error) {
        (UiLocale::ZhCn, Some(error)) => {
            format!("自动下载并安装 Microsoft Edge WebView2 失败：\n{error}\n\n")
        }
        (UiLocale::ZhCn, None) => {
            "安装程序已结束，但重新检测仍未发现 Microsoft Edge WebView2。\n\n".to_string()
        }
        (UiLocale::EnUs, Some(error)) => format!(
            "Microsoft Edge WebView2 could not be downloaded and installed automatically:\n{error}\n\n"
        ),
        (UiLocale::EnUs, None) => {
            "The installer finished, but Microsoft Edge WebView2 is still unavailable.\n\n"
                .to_string()
        }
    };
    let remedy = match locale {
        UiLocale::ZhCn => {
            "请检查网络连接后重试，或前往 <a href=\"https://aka.ms/webview2installer\">https://aka.ms/webview2installer</a> 手动下载并安装 Evergreen Bootstrapper。安装完成后重新打开 OEA。"
        }
        UiLocale::EnUs => {
            "Check your network connection and try again, or manually download and install the Evergreen Bootstrapper from <a href=\"https://aka.ms/webview2installer\">https://aka.ms/webview2installer</a>. Then restart OEA."
        }
    };
    format!("{reason}{remedy}")
}

/// 下载引导程序到应用缓存目录并安装，成功后清理临时文件。
fn install(cache_dir: &Path) -> Result<()> {
    let exe_path = download_bootstrapper(cache_dir)?;
    let result = run_bootstrapper(&exe_path);
    // 无论成败都清理临时文件；清理失败只记日志，不影响安装结果
    if let Err(e) = fs::remove_file(&exe_path) {
        warn!("清理 WebView2 引导程序失败: {e}");
    }
    result
}

/// 用 `reqwest` 从微软官方链接下载 Evergreen Bootstrapper 到应用缓存目录。
fn download_bootstrapper(cache_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).context("创建应用缓存目录失败")?;
    let exe_path = cache_dir.join(format!(
        "MicrosoftEdgeWebview2Setup_{}.exe",
        std::process::id()
    ));

    info!("下载 WebView2 引导程序: {BOOTSTRAPPER_URL}");
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .context("创建 HTTP 客户端失败")?;

    let mut response = client
        .get(BOOTSTRAPPER_URL)
        .send()
        .context("网络请求失败")?;
    if !response.status().is_success() {
        bail!("服务器返回错误: HTTP {}", response.status());
    }

    let mut file = fs::File::create(&exe_path).context("创建临时文件失败")?;
    response.copy_to(&mut file).context("写入引导程序失败")?;
    file.flush().context("刷新文件缓冲失败")?;

    // 基本完整性检查：至少 1MB（官方引导程序约 2MB），避免下载到错误页
    let size = fs::metadata(&exe_path)
        .context("读取引导程序文件信息失败")?
        .len();
    if size < 1024 * 1024 {
        bail!("下载的引导程序文件异常偏小（{size} 字节），已中止安装");
    }

    Ok(exe_path)
}

/// 运行引导程序并等待其退出。
fn run_bootstrapper(exe_path: &std::path::Path) -> Result<()> {
    info!("运行 WebView2 引导程序: {}", exe_path.display());
    let status = Command::new(exe_path)
        // .args(["/silent", "/install"])
        // .creation_flags(CREATE_NO_WINDOW)
        .status()
        .context("启动 WebView2 安装程序失败")?;

    if status.success() {
        info!("WebView2 引导程序正常退出");
        Ok(())
    } else {
        bail!("WebView2 安装程序退出码: {status}")
    }
}
