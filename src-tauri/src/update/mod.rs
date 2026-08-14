//! 自动更新：下载、进度、取消、sha256 校验与系统代理解析。
//!
//! 职责边界（与设计文档 v4 一致）：
//! - 前端负责决策与编排（下载源、时机、安装流程）；
//! - 本模块只做"重活"：流式下载到 `<root>/cache/downloads/`，上报进度，
//!   支持取消与 sha256 校验，并解析 Windows 系统代理供检查请求与下载共用。

pub mod install;

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;
use tracing::{info, warn};

/// 下载进度事件（前端按 `session_id` 过滤旧任务的迟到事件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    /// 下载会话编号（自增，用于区分新旧任务）
    pub session_id: u64,
    /// 已下载字节数
    pub downloaded_size: u64,
    /// 总字节数（未知时为 `0`）
    pub total_size: u64,
    /// EMA 平滑后的瞬时速度（字节/秒）
    pub speed: u64,
    /// 进度百分比（`0.0` ~ `100.0`，总大小未知时为 `0.0`）
    pub progress: f64,
}

/// 下载结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    /// 本次下载的会话编号
    pub session_id: u64,
    /// 实际保存路径（可能因 `Content-Disposition` / 重定向 URL 检测到真实文件名而不同于入参）
    pub actual_save_path: String,
    /// 检测到的文件名（未检测到时为 `None`）
    pub detected_filename: Option<String>,
}

/// 全局下载取消标志（每次下载开始前重置；前端模块级互斥保证同一时间只有一个下载）。
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);
/// 当前下载会话编号（每次下载自增，旧任务的进度事件与取消请求据此失效）。
static CURRENT_DOWNLOAD_SESSION: AtomicU64 = AtomicU64::new(0);
/// 已下载字节数（仅作进度采样的共享计数，允许最终一致，用 `Relaxed` 序即可）。
static DOWNLOADED_BYTES: AtomicU64 = AtomicU64::new(0);
/// 安装进行中标志：安装期间拒绝退出（`quit` / 窗口关闭 / 托盘退出统一检查）。
static UPDATE_INSTALLING: AtomicBool = AtomicBool::new(false);

/// 当前是否正在安装更新。
pub fn is_installing() -> bool {
    UPDATE_INSTALLING.load(Ordering::SeqCst)
}

/// 设置安装进行中标志（前端在安装开始/结束时调用）。
#[tauri::command]
pub fn set_update_installing(installing: bool) {
    UPDATE_INSTALLING.store(installing, Ordering::SeqCst);
    info!("设置更新安装状态: {installing}");
}

/// 临时文件守卫：下载异常退出时同步删除 `.downloading` 半成品，成功重命名后调用 `disarm`。
struct TempFileGuard {
    path: Option<PathBuf>,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// 成功重命名后调用，使 drop 时不再尝试删除（文件已移至目标路径）。
    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            // 必须同步删除：异步删除可能在下一次下载创建同名临时文件后才执行，导致误删。
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// 进度上报任务守卫：在函数任意返回路径上都能停止采样循环。
struct ProgressEmitterGuard(Option<tokio::sync::oneshot::Sender<()>>);

impl Drop for ProgressEmitterGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

/// 解析 Windows 系统代理（读注册表 `Internet Settings`）。
///
/// 返回可直接交给 reqwest 的 `http://host:port`；未启用或格式无法解析时返回 `Ok(None)`。
fn resolve_system_proxy_inner() -> Result<Option<String>, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    let root = RegKey::predef(HKEY_CURRENT_USER);
    let key = match root.open_subkey(INTERNET_SETTINGS) {
        Ok(key) => key,
        Err(error) => {
            warn!("打开系统代理注册表失败: {error}");
            return Ok(None);
        }
    };

    let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return Ok(None);
    }

    let server: String = key.get_value("ProxyServer").unwrap_or_default();
    let Some(proxy) = normalize_proxy_server(&server) else {
        warn!("系统代理 ProxyServer 格式无法解析: {server:?}");
        return Ok(None);
    };
    info!("解析到系统代理: {proxy}");
    Ok(Some(proxy))
}

/// 归一化注册表 `ProxyServer` 值：
/// - `host:port` → `http://host:port`
/// - `http=host:port;https=host2:port` → 优先 `https=`，否则 `http=`
/// - 仅含 `socks=` 或无法解析时返回 `None`（reqwest 未启用 socks 特性）
fn normalize_proxy_server(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.contains('=') {
        let mut chosen: Option<&str> = None;
        for part in raw.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let Some((scheme, addr)) = part.split_once('=') else {
                continue;
            };
            let addr = addr.trim();
            if addr.is_empty() {
                continue;
            }
            if matches!(scheme.trim(), "http" | "https") {
                chosen = Some(addr);
                if scheme.trim() == "https" {
                    break;
                }
            }
        }
        return chosen.map(|addr| format!("http://{addr}"));
    }

    Some(format!("http://{raw}"))
}

/// 按代理模式构建 HTTP 客户端。
///
/// - `none`：直连（显式 `no_proxy`）；
/// - `system`：解析系统代理后使用；
/// - `custom`：使用 `proxy_url`。
fn build_client(
    user_agent: &str,
    proxy_mode: Option<String>,
    proxy_url: Option<String>,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30 * 60))
        // GitHub asset 端点会 302 到签名 CDN 地址，需要跟随重定向；
        // 默认策略会跨主机丢弃 `Authorization`，不会把 token 泄露给 CDN。
        .redirect(reqwest::redirect::Policy::limited(10));

    match proxy_mode.as_deref().unwrap_or("none") {
        "system" => {
            if let Some(url) = resolve_system_proxy_inner()? {
                builder = builder.proxy(
                    reqwest::Proxy::all(url.as_str())
                        .map_err(|e| format!("系统代理配置失败: {e}"))?,
                );
            }
        }
        "custom" => {
            if let Some(url) = proxy_url.filter(|url| !url.trim().is_empty()) {
                builder = builder.proxy(
                    reqwest::Proxy::all(url.as_str()).map_err(|e| format!("代理配置失败: {e}"))?,
                );
            }
        }
        _ => {
            builder = builder.no_proxy();
        }
    }

    builder
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

/// 将字节切片编码为小写十六进制字符串。
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// 简单百分号解码（`%XX`），非 UTF-8 字节以 `U+FFFD` 替换。
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 清理文件名，防止目录穿越：只保留最后一段、拒绝 `..` 与空名、要求含扩展名。
fn sanitize_filename(filename: &str) -> Option<String> {
    let name = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    if name.is_empty() || name == "." || name == ".." || name.starts_with("..") {
        return None;
    }
    if !name.contains('.') {
        return None;
    }
    Some(name.to_string())
}

/// 解析 `Content-Disposition` 中的文件名（`filename*=` RFC 5987 优先，`filename=` 兜底）。
fn parse_content_disposition(header: &str) -> Option<String> {
    let header_lower = header.to_lowercase();

    // 优先 `filename*=`（RFC 5987 编码）
    if let Some(start) = header_lower.find("filename*=") {
        let rest = &header[start + 10..];
        if let Some(quote_pos) = rest.find("''") {
            let encoded = rest[quote_pos + 2..].split(';').next().unwrap_or("").trim();
            let decoded = percent_decode(encoded.trim_matches('"'));
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }

    // 兜底 `filename=`（排除 `filename*=`）
    let mut search_start = 0;
    while let Some(pos) = header_lower[search_start..].find("filename=") {
        let absolute = search_start + pos;
        if absolute > 0 && header.as_bytes().get(absolute - 1) == Some(&b'*') {
            search_start = absolute + 9;
            continue;
        }
        let rest = &header[absolute + 9..];
        let name = rest
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();
        if !name.is_empty() {
            return Some(name);
        }
        break;
    }
    None
}

/// 从响应中提取文件名：优先 `Content-Disposition`，其次 302 重定向后的最终 URL 路径。
fn extract_filename_from_response(response: &reqwest::Response) -> Option<String> {
    if let Some(cd) = response.headers().get("content-disposition") {
        if let Ok(cd_str) = cd.to_str() {
            if let Some(name) = parse_content_disposition(cd_str) {
                if let Some(safe) = sanitize_filename(&name) {
                    return Some(safe);
                }
            }
        }
    }

    let path = response.url().path();
    if let Some(last_segment) = path.rsplit('/').next() {
        if let Some(safe) = sanitize_filename(&percent_decode(last_segment)) {
            return Some(safe);
        }
    }
    None
}

/// 流式下载文件，支持进度事件、取消与 sha256 校验。
///
/// - 临时文件写入 `{save_path}.{session_id}.downloading`，成功后原子重命名；
/// - 校验失败 / 网络错误 / 取消时，`TempFileGuard` 负责删除半成品；
/// - 进度事件 `download-progress` 每 100ms 上报一次，带 `session_id` 供前端过滤；
/// - `accept` 由前端按下载源传入（GitHub 资产端点需要 `application/octet-stream`），
///   配合 `auth_token` 实现 private 仓库的资产下载；客户端自动跟随 302 重定向。
#[allow(clippy::too_many_arguments)] // 参数多但均为简单值，封装成结构体反而降低可读性
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    url: String,
    save_path: String,
    total_size: Option<u64>,
    expected_sha256: Option<String>,
    proxy_mode: Option<String>,
    proxy_url: Option<String>,
    auth_token: Option<String>,
    accept: Option<String>,
    user_agent: Option<String>,
) -> Result<DownloadResult, String> {
    let session_id = CURRENT_DOWNLOAD_SESSION.fetch_add(1, Ordering::SeqCst) + 1;
    DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);
    DOWNLOADED_BYTES.store(0, Ordering::SeqCst);
    info!("download_update: session={session_id} url={url} -> {save_path}");

    let save_path_obj = Path::new(&save_path);
    if let Some(parent) = save_path_obj.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建下载目录: {e}"))?;
    }

    // UA 由前端基于 `@tauri-apps/plugin-os` 生成并传入（唯一来源）；
    // 兜底仅覆盖未传入 UA 的旧调用方。
    let user_agent = user_agent.unwrap_or_else(|| format!("OEA/{}", app.package_info().version));
    let client = build_client(&user_agent, proxy_mode, proxy_url)?;

    let mut request = client.get(&url);
    if let Some(token) = auth_token.filter(|token| !token.trim().is_empty()) {
        request = request.header("Authorization", format!("token {}", token.trim()));
    }
    if let Some(accept) = accept.filter(|accept| !accept.trim().is_empty()) {
        request = request.header(reqwest::header::ACCEPT, accept.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let detected_filename = extract_filename_from_response(&response);
    let actual_save_path = if let Some(ref name) = detected_filename {
        save_path_obj
            .parent()
            .map(|parent| parent.join(name))
            .unwrap_or_else(|| PathBuf::from(name))
    } else {
        save_path_obj.to_path_buf()
    };

    let temp_path = format!("{}.{}.downloading", actual_save_path.display(), session_id);
    let mut temp_guard = TempFileGuard::new(PathBuf::from(&temp_path));

    let total = total_size
        .filter(|size| *size > 0)
        .or_else(|| response.content_length())
        .unwrap_or(0);

    // 有界通道将网络读取与磁盘写入解耦，避免大文件撑爆内存
    let (write_tx, write_rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(64);
    let temp_path_for_writer = temp_path.clone();
    let write_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut write_rx = write_rx;
        let file = std::fs::File::create(&temp_path_for_writer)
            .map_err(|e| format!("无法创建临时文件: {e}"))?;
        let mut writer = std::io::BufWriter::with_capacity(512 * 1024, file);
        while let Some(chunk) = write_rx.blocking_recv() {
            writer
                .write_all(&chunk)
                .map_err(|e| format!("写入文件失败: {e}"))?;
        }
        writer
            .flush()
            .map_err(|e| format!("刷新写入缓冲区失败: {e}"))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|e| format!("同步文件失败: {e}"))?;
        Ok(())
    });

    // 独立进度上报任务，避免下载热路径被 emit 阻塞
    let app_for_emitter = app.clone();
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    let progress_guard = ProgressEmitterGuard(Some(stop_tx));
    tokio::spawn(async move {
        let mut last_downloaded = 0u64;
        let mut last_instant = tokio::time::Instant::now();
        let mut smoothed_speed: f64 = 0.0;
        const EMA_ALPHA: f64 = 0.3;
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    let downloaded = DOWNLOADED_BYTES.load(Ordering::Relaxed);
                    let now = tokio::time::Instant::now();
                    let elapsed = now.duration_since(last_instant);
                    if elapsed.as_millis() == 0 {
                        continue;
                    }
                    let bytes_in_interval = downloaded.saturating_sub(last_downloaded);
                    let instant_speed = bytes_in_interval as f64 / elapsed.as_secs_f64();
                    smoothed_speed = if smoothed_speed == 0.0 {
                        instant_speed
                    } else {
                        EMA_ALPHA * instant_speed + (1.0 - EMA_ALPHA) * smoothed_speed
                    };
                    let progress = if total > 0 {
                        ((downloaded as f64 / total as f64) * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    let _ = app_for_emitter.emit(
                        "download-progress",
                        DownloadProgressEvent {
                            session_id,
                            downloaded_size: downloaded,
                            total_size: total,
                            speed: smoothed_speed as u64,
                            progress,
                        },
                    );
                    last_downloaded = downloaded;
                    last_instant = now;
                }
            }
        }
    });

    // 网络循环 + 流式 sha256（校验不额外增加 IO）
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut download_error: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        if DOWNLOAD_CANCELLED.load(Ordering::SeqCst)
            || CURRENT_DOWNLOAD_SESSION.load(Ordering::SeqCst) != session_id
        {
            download_error = Some("下载已取消".to_string());
            break;
        }
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                download_error = Some(format!("下载数据失败: {e}"));
                break;
            }
        };
        hasher.update(&chunk);
        let len = chunk.len() as u64;
        if write_tx.send(chunk).await.is_err() {
            download_error = Some("磁盘写入线程异常退出".to_string());
            break;
        }
        downloaded += len;
        DOWNLOADED_BYTES.store(downloaded, Ordering::Relaxed);
    }

    // 收尾前再检查一次取消标志
    if download_error.is_none()
        && (DOWNLOAD_CANCELLED.load(Ordering::SeqCst)
            || CURRENT_DOWNLOAD_SESSION.load(Ordering::SeqCst) != session_id)
    {
        download_error = Some("下载已取消".to_string());
    }

    // 关闭发送端，通知写入线程结束
    drop(write_tx);
    let write_result = write_handle
        .await
        .map_err(|e| format!("写入任务异常: {e}"))?;

    if let Some(error) = download_error {
        // 写入线程通常持有更具体的 I/O 错误（如磁盘满），优先返回
        if let Err(write_error) = write_result {
            return Err(write_error);
        }
        return Err(error);
    }
    write_result?;

    // sha256 校验（允许带 `sha256:` 前缀）
    let actual_hash = hex_encode(hasher.finalize().as_slice());
    if let Some(expected) = expected_sha256 {
        let expected = expected
            .trim()
            .trim_start_matches("sha256:")
            .to_ascii_lowercase();
        if !expected.is_empty() && actual_hash != expected {
            return Err(format!(
                "sha256 校验失败：期望 {expected}，实际 {actual_hash}"
            ));
        }
    }
    info!("sha256 校验通过: {actual_hash}");

    // 发送最终进度
    let _ = app.emit(
        "download-progress",
        DownloadProgressEvent {
            session_id,
            downloaded_size: downloaded,
            total_size: if total > 0 { total } else { downloaded },
            speed: 0,
            progress: 100.0,
        },
    );

    std::fs::rename(&temp_path, &actual_save_path)
        .map_err(|e| format!("重命名临时文件失败: {e}"))?;
    temp_guard.disarm();

    info!(
        "download_update 完成: {downloaded} 字节 -> {} (session {session_id})",
        actual_save_path.display()
    );
    drop(progress_guard);

    Ok(DownloadResult {
        session_id,
        actual_save_path: actual_save_path.to_string_lossy().into_owned(),
        detected_filename,
    })
}

/// 取消当前下载（置标志即可，临时文件由守卫清理）。
#[tauri::command]
pub fn cancel_download() {
    DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
    info!("收到取消下载请求");
}

/// 返回更新包下载目录（`<root>/cache/downloads`），不存在时创建。
#[tauri::command]
pub fn get_update_download_dir() -> Result<String, String> {
    let dir = crate::app_paths::AppPaths::new()
        .map_err(|e| format!("无法定位应用根目录: {e}"))?
        .cache_dir()
        .join("downloads");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("无法创建下载目录 {}: {e}", dir.display()))?;
    Ok(dir.to_string_lossy().into_owned())
}

/// 解析 Windows 系统代理（前端检查请求与 Rust 下载共用）。
#[tauri::command]
pub fn resolve_system_proxy() -> Result<Option<String>, String> {
    resolve_system_proxy_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("OEA-windows-x86_64-v0.1.0.zip"),
            Some("OEA-windows-x86_64-v0.1.0.zip".to_string())
        );
        assert_eq!(sanitize_filename(".."), None);
        assert_eq!(sanitize_filename("a/b/c.zip"), Some("c.zip".to_string()));
        assert_eq!(sanitize_filename("noext"), None);
        assert_eq!(sanitize_filename(""), None);
    }

    #[test]
    fn test_parse_content_disposition() {
        assert_eq!(
            parse_content_disposition("attachment; filename=\"a.zip\""),
            Some("a.zip".to_string())
        );
        assert_eq!(
            parse_content_disposition("attachment; filename*=UTF-8''a%20b.zip"),
            Some("a b.zip".to_string())
        );
        assert_eq!(
            parse_content_disposition("attachment; filename=a.zip; size=1"),
            Some("a.zip".to_string())
        );
        assert_eq!(parse_content_disposition("attachment"), None);
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn test_normalize_proxy_server() {
        assert_eq!(
            normalize_proxy_server("127.0.0.1:7890"),
            Some("http://127.0.0.1:7890".to_string())
        );
        assert_eq!(
            normalize_proxy_server("http=127.0.0.1:7890;https=127.0.0.1:7891"),
            Some("http://127.0.0.1:7891".to_string())
        );
        assert_eq!(normalize_proxy_server("socks=127.0.0.1:1080"), None);
        assert_eq!(normalize_proxy_server(""), None);
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0xab, 0x0f, 0x12]), "ab0f12");
    }
}
