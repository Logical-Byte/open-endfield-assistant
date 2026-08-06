//! 单次扫描当前档案详情（分号键 / 前端「单次扫描」触发）。
//!
//! 仅截屏识别当前档案详情的标题并记录日志，**不做任何鼠标键盘输入操作**。

use anyhow::Result;
use tracing::{info, warn};

use crate::{
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
};

use super::constants::OCR_ROI;
use super::result::{ScanReporter, encode_png_data_url};

/// 扫描当前档案详情的标题。
///
/// 如果检测到不在档案详情页面，仅记录警告并返回，不做任何操作。
pub fn single_scan(
    session: &mut Session,
    scene_manager: &SceneManager,
    reporter: &ScanReporter,
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
    let ocr_text = match session.ocr_in_roi(&screenshot, OCR_ROI) {
        Ok(text) if !text.trim().is_empty() => {
            info!("当前档案标题：{}", text.trim());
            text.trim().to_string()
        }
        Ok(_) => {
            info!("当前档案标题：（空）");
            String::new()
        }
        Err(e) => {
            // OCR 失败不中断，记录警告并给出提示
            warn!("OCR 识别失败: {e:#}");
            info!("当前档案标题：（OCR 识别失败）");
            String::new()
        }
    };

    // 3. 上报结果（单次扫描无法确定具体分类，大类 / 小类 id 留空、不纠错）
    let status = if ocr_text.is_empty() {
        "failed"
    } else {
        "success"
    };
    reporter.report(
        status,
        "",
        "",
        encode_png_data_url(&screenshot),
        ocr_text,
        None,
    );
    Ok(())
}
