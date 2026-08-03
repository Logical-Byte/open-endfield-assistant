//! 单子分类内扫描档案的循环逻辑。
//!
//! 进入一个子分类后，依次扫描该分类中的所有档案：
//! 1. OCR 识别档案标题 → 记录 SUCCESS 日志
//! 2. 尝试翻到下一份档案（"下一篇"按钮 或 "档案详情右箭头"按钮）
//! 3. 两个都翻不动 → 扫描完毕

use anyhow::Result;
use tracing::debug;

use crate::{
    scan_result::{ScanResult, encode_png_data_url},
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
    success,
};

use super::constants::{ARROW_RIGHT_ROI, CLOSE_BUTTON_ROI, NEXT_BUTTON_ROI, OCR_ROI, THRESHOLD};

/// 扫描当前子界面中的所有档案。
///
/// # 前置条件
/// - `session` 当前处于档案库子界面
/// - `category` 为该子界面所属的档案库分类（音像存档 / 见闻辑录 / 中枢档案）
///
/// # 工作流程
/// 1. 点击第 1 份档案 (401, 182) 进入档案详情页面
/// 2. 循环：OCR 标题 → 上报扫描结果 → 翻到下一篇 → 直到翻不动
/// 3. 点击关闭返回子界面
pub fn scan_current_sub_scene(
    session: &mut Session,
    scene_manager: &SceneManager,
    category: &'static str,
) -> Result<()> {
    // 1. 点击第 1 份档案进入档案详情页面
    debug!("点击第 1 份档案 (401, 182)");
    session.click_at_720p(401, 182)?;

    // 等待详情页面加载
    std::thread::sleep(std::time::Duration::from_millis(800));

    // 验证是否进入了详情页面
    let arrived = scene_manager.wait_for_scene(SceneId::档案详情页面, session, 15)?;
    if !arrived {
        anyhow::bail!("未能进入档案详情页面，可能该子分类没有档案");
    }

    // 2. 循环扫描所有档案
    let mut archive_count = 0u32;
    loop {
        archive_count += 1;

        // 2a. OCR 识别档案标题
        let screenshot = session.screencap_for_recognition()?;
        let ocr_text = match session.ocr_in_roi(&screenshot, OCR_ROI) {
            Ok(text) if !text.trim().is_empty() => {
                success!("第 {} 份档案标题：{}", archive_count, text.trim());
                text.trim().to_string()
            }
            Ok(_) => {
                success!("第 {} 份档案标题：（空）", archive_count);
                String::new()
            }
            Err(e) => {
                // OCR 失败不中断流程，记录日志继续
                debug!("OCR 识别失败（第 {archive_count} 份）: {e:#}");
                success!("第 {} 份档案标题：（OCR 识别失败）", archive_count);
                String::new()
            }
        };

        // 2b. 把识别结果推送给前端（卡片展示：状态 / 序号 / 分类 / 详情截图 / 可编辑文本）
        // 目前只要 OCR 结果非空就视为识别成功
        let status = if ocr_text.is_empty() {
            "failed"
        } else {
            "success"
        };
        session.emit_scan_result(ScanResult {
            status: status.to_string(),
            index: session.next_scan_index(),
            category: category.to_string(),
            image: encode_png_data_url(&screenshot),
            ocr_result: ocr_text,
        });

        // 2c. 尝试翻到下一篇
        let screenshot = session.screencap_for_recognition()?;

        // 先尝试 "下一篇" 按钮
        if session.find_and_click_template(
            &screenshot,
            "情报档案库/下一篇.png",
            NEXT_BUTTON_ROI,
            THRESHOLD,
        )? {
            debug!("点击「下一篇」，进入第 {} 份档案", archive_count + 1);
            // 等待详情页面切换
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }

        // 再尝试 "档案详情右箭头" 按钮
        if session.find_and_click_template(
            &screenshot,
            "情报档案库/档案详情右箭头.png",
            ARROW_RIGHT_ROI,
            THRESHOLD,
        )? {
            debug!(
                "点击「档案详情右箭头」，进入第 {} 份档案",
                archive_count + 1
            );
            // 等待详情页面切换
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }

        // 两个按钮都没找到 → 扫描完毕
        debug!("「下一篇」和「档案详情右箭头」均未找到，扫描完毕（共 {archive_count} 份）");
        break;
    }

    // 3. 点击关闭按钮返回子界面
    debug!("点击档案详情关闭按钮返回子界面");
    let screenshot = session.screencap_for_recognition()?;
    if !session.find_and_click_template(
        &screenshot,
        "情报档案库/档案详情关闭.png",
        CLOSE_BUTTON_ROI,
        THRESHOLD,
    )? {
        // 如果模板匹配失败，直接点击右上角固定位置
        debug!("模板匹配关闭按钮失败，尝试固定坐标点击");
        session.click_at_720p(1240, 50)?;
    }

    // 等待返回子界面
    std::thread::sleep(std::time::Duration::from_millis(800));

    Ok(())
}
