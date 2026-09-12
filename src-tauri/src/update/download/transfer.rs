use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};

use super::target::DownloadTarget;
use crate::update::manager::DownloadSession;

pub(super) struct DownloadSummary {
    /// 原子发布后的最终文件路径。
    pub(super) path: PathBuf,
    /// 从响应体读取并写入文件的字节数。
    pub(super) downloaded_size: u64,
    /// 响应体内容的小写十六进制 SHA-256。
    pub(super) sha256: String,
}

struct TransferSummary {
    downloaded_size: u64,
    sha256: String,
}

/// 将响应体下载到目标文件，校验内容并原子发布。
pub(super) async fn download_to_target(
    response: reqwest::Response,
    target: DownloadTarget,
    session: Arc<DownloadSession>,
    expected_sha256: Option<&str>,
) -> Result<DownloadSummary, String> {
    let transfer = response_to_file(response, target.staging_path(), session).await?;
    verify_sha256(&transfer.sha256, expected_sha256)?;
    let path = target.publish()?;

    Ok(DownloadSummary {
        path,
        downloaded_size: transfer.downloaded_size,
        sha256: transfer.sha256,
    })
}

/// 通过有界队列将响应体流式传给阻塞文件写入任务。
async fn response_to_file(
    response: reqwest::Response,
    output_path: &Path,
    session: Arc<DownloadSession>,
) -> Result<TransferSummary, String> {
    let (tx, rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(64);
    let output_path = output_path.to_path_buf();
    let writer_task = tokio::task::spawn_blocking(move || write_file(&output_path, rx));

    let transfer_result = stream_response(response, tx, session).await;
    let writer_result = writer_task
        .await
        .map_err(|error| format!("写入任务异常: {error}"))?;

    // 磁盘已满等具体 I/O 错误优先于同时发生的响应流错误。
    writer_result.and(transfer_result)
}

async fn stream_response(
    response: reqwest::Response,
    tx: tokio::sync::mpsc::Sender<bytes::Bytes>,
    session: Arc<DownloadSession>,
) -> Result<TransferSummary, String> {
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        if session.is_cancelled() {
            return Err("下载已取消".to_string());
        }

        let chunk = chunk.map_err(|error| format!("下载数据失败: {error}"))?;

        hasher.update(&chunk);
        let chunk_size = chunk.len() as u64;
        tx.send(chunk)
            .await
            .map_err(|_| "磁盘写入线程异常退出".to_string())?;
        downloaded += chunk_size;
        session.set_downloaded_bytes(downloaded);
    }

    if session.is_cancelled() {
        return Err("下载已取消".to_string());
    }

    Ok(TransferSummary {
        downloaded_size: downloaded,
        sha256: hex_encode(hasher.finalize().as_slice()),
    })
}

fn write_file(
    output_path: &Path,
    mut rx: tokio::sync::mpsc::Receiver<bytes::Bytes>,
) -> Result<(), String> {
    let file =
        std::fs::File::create(output_path).map_err(|error| format!("无法创建输出文件: {error}"))?;
    let mut writer = std::io::BufWriter::with_capacity(512 * 1024, file);
    while let Some(chunk) = rx.blocking_recv() {
        writer
            .write_all(&chunk)
            .map_err(|error| format!("写入文件失败: {error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("刷新写入缓冲区失败: {error}"))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| format!("同步文件失败: {error}"))?;
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn verify_sha256(actual: &str, expected: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let expected = expected
        .trim()
        .trim_start_matches("sha256:")
        .to_ascii_lowercase();
    if !expected.is_empty() && actual != expected {
        return Err(format!("sha256 校验失败：期望 {expected}，实际 {actual}"));
    }
    Ok(())
}
