//! 单次扫描当前档案详情（分号键触发）。
//!
//! 仅截屏识别当前档案详情的标题并记录日志，**不做任何鼠标键盘输入操作**。

use anyhow::Result;
use tracing::warn;

use crate::{
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
    success,
};

use super::constants::OCR_ROI;

/// 扫描当前档案详情的标题。
///
/// # 前置条件
/// 假定当前位于任意档案详情页面。
///
/// 如果检测到不在档案详情页面，仅记录警告并返回，不做任何操作。
pub fn scan_single_archive_detail(
    session: &mut Session,
    scene_manager: &SceneManager,
) -> Result<()> {
    // 1. 检测当前场景是否为档案详情页面
    let current = scene_manager.detect_current_scene(session)?;
    if current != SceneId::档案详情页面 {
        warn!(
            "当前不在档案详情页面（检测到场景: {:?}），跳过单次扫描",
            current
        );
        return Ok(());
    }

    // 2. OCR 识别档案标题并记录日志
    let screenshot = session.screencap_for_recognition()?;
    match session.ocr_in_roi(&screenshot, OCR_ROI) {
        Ok(text) if !text.trim().is_empty() => {
            success!("当前档案标题：{}", text.trim());
        }
        Ok(_) => {
            success!("当前档案标题：（空）");
        }
        Err(e) => {
            // OCR 失败不中断，记录警告并给出提示
            warn!("OCR 识别失败: {e:#}");
            success!("当前档案标题：（OCR 识别失败）");
        }
    }
    Ok(())
}
