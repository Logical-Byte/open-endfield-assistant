//! 窗口管理 implementation。目标专属流程只依赖共用几何、激活和输入能力。

mod activation;
mod chromium;
mod geometry;
mod hdr;
mod search;
mod unity;

pub(crate) use activation::{is_foreground, request_activation, restore_if_minimized};
pub(crate) use chromium::resize_viewport;
pub(crate) use geometry::{adjust_client_to_work_area, client_size, resize_client};
pub(crate) use hdr::is_hdr_enabled_on_window_monitor;
pub(crate) use search::search;
pub(crate) use unity::ensure_windowed as ensure_unity_windowed;

use crate::{ContentKind, ContentRect, Window};
use std::{
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const PREPARE_TIMEOUT: Duration = Duration::from_secs(5);
const SIZE_SETTLE_TIME: Duration = Duration::from_millis(300);

pub(crate) fn content_rect(window: Window, kind: ContentKind) -> anyhow::Result<ContentRect> {
    match kind {
        ContentKind::Client => {
            let (width, height) = geometry::client_size(window)?;
            Ok(ContentRect {
                x: 0,
                y: 0,
                width: width as u32,
                height: height as u32,
            })
        }
        ContentKind::BrowserViewport => chromium::viewport_rect(window),
    }
}

/// 反复求值直到返回 `true`，单次求值返回后检查总等待时间。
fn wait_until(mut ready: impl FnMut() -> anyhow::Result<bool>) -> anyhow::Result<()> {
    let started = Instant::now();
    loop {
        if ready()? {
            return Ok(());
        }
        anyhow::ensure!(started.elapsed() < PREPARE_TIMEOUT, "等待窗口调整超时");
        thread::sleep(POLL_INTERVAL);
    }
}
