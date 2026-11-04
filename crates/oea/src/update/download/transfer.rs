use crate::update;
use crate::update::error::Reason;
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
) -> Result<DownloadSummary, update::Error> {
    let transfer = response_to_file(response, target.staging_path(), Arc::clone(&session)).await?;
    verify_sha256(&transfer.sha256, expected_sha256)?;
    if session.cancellation().is_cancelled() {
        return Err(update::Error::new(Reason::Cancelled));
    }
    let path = target
        .publish()
        .map_err(|error| update::Error::failed(Reason::FileAccess, anyhow::anyhow!(error)))?;

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
) -> Result<TransferSummary, update::Error> {
    let (tx, rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(64);
    let output_path = output_path.to_path_buf();
    let writer_task = tokio::task::spawn_blocking(move || write_file(&output_path, rx));
    let cancellation = session.cancellation();

    let transfer_result = stream_response(response, tx, session).await;
    let writer_result = writer_task.await.map_err(|error| {
        update::Error::failed(
            Reason::FileAccess,
            anyhow::Error::new(error).context("写入任务异常"),
        )
    })?;

    // 磁盘已满等具体 I/O 错误优先于同时发生的响应流错误。
    writer_result?;
    let transfer = transfer_result?;
    if cancellation.is_cancelled() {
        return Err(update::Error::new(Reason::Cancelled));
    }
    Ok(transfer)
}

async fn stream_response(
    response: reqwest::Response,
    tx: tokio::sync::mpsc::Sender<bytes::Bytes>,
    session: Arc<DownloadSession>,
) -> Result<TransferSummary, update::Error> {
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let cancellation = session.cancellation();

    loop {
        let chunk = tokio::select! {
            biased;
            chunk = stream.next() => chunk,
            _ = cancellation.cancelled() => return Err(update::Error::new(Reason::Cancelled)),
        };
        let Some(chunk) = chunk else {
            break;
        };
        let chunk = chunk.map_err(|error| {
            update::Error::failed(
                Reason::Network,
                anyhow::Error::new(error).context("下载数据失败"),
            )
        })?;

        hasher.update(&chunk);
        let chunk_size = chunk.len() as u64;
        tokio::select! {
            biased;
            result = tx.send(chunk) => {
                result.map_err(|_| update::Error::failed(Reason::FileAccess, anyhow::anyhow!("磁盘写入线程异常退出")))?;
            }
            _ = cancellation.cancelled() => return Err(update::Error::new(Reason::Cancelled)),
        }
        downloaded += chunk_size;
        session.set_downloaded_bytes(downloaded);
    }

    Ok(TransferSummary {
        downloaded_size: downloaded,
        sha256: hex_encode(hasher.finalize().as_slice()),
    })
}

fn write_file(
    output_path: &Path,
    mut rx: tokio::sync::mpsc::Receiver<bytes::Bytes>,
) -> Result<(), update::Error> {
    let file = std::fs::File::create(output_path).map_err(|error| {
        update::Error::failed(
            Reason::FileAccess,
            anyhow::Error::new(error).context("无法创建输出文件"),
        )
    })?;
    let mut writer = std::io::BufWriter::with_capacity(512 * 1024, file);
    while let Some(chunk) = rx.blocking_recv() {
        writer.write_all(&chunk).map_err(|error| {
            update::Error::failed(
                Reason::FileAccess,
                anyhow::Error::new(error).context("写入文件失败"),
            )
        })?;
    }
    writer.flush().map_err(|error| {
        update::Error::failed(
            Reason::FileAccess,
            anyhow::Error::new(error).context("刷新写入缓冲区失败"),
        )
    })?;
    writer.get_ref().sync_all().map_err(|error| {
        update::Error::failed(
            Reason::FileAccess,
            anyhow::Error::new(error).context("同步文件失败"),
        )
    })?;
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

fn verify_sha256(actual: &str, expected: Option<&str>) -> Result<(), update::Error> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let expected = expected
        .trim()
        .trim_start_matches("sha256:")
        .to_ascii_lowercase();
    if !expected.is_empty() && actual != expected {
        return Err(update::Error::failed(
            Reason::Integrity,
            anyhow::anyhow!("sha256 校验失败：期望 {expected}，实际 {actual}"),
        ));
    }
    Ok(())
}
