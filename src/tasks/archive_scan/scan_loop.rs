//! 单子分类内扫描档案的循环逻辑。
//!
//! 进入一个子分类后，依次扫描该分类中的所有档案：
//! 1. OCR 识别档案标题 → 记录 SUCCESS 日志
//! 2. 尝试翻到下一份档案（"下一篇"按钮 或 "档案详情右箭头"按钮）
//! 3. 两个都翻不动 → 扫描完毕

use anyhow::Result;
use tracing::debug;

use crate::{
    ltwh,
    scene::{SceneId, scene_manager::SceneManager},
    session::Session,
    success,
    utils::region::Region2D,
};

/// OCR 识别区域（720p 基准 ltwh）：档案标题位置
const OCR_ROI: Region2D<u32> = ltwh!(350, 58, 578, 42);

/// "下一篇" 按钮搜索区域（720p 基准 ltrb）
const NEXT_BUTTON_ROI: Region2D<u32> = Region2D::from_ltrb(762, 654, 925, 711);

/// "档案详情右箭头" 搜索区域（720p 基准 ltrb）
const ARROW_RIGHT_ROI: Region2D<u32> = Region2D::from_ltrb(1206, 313, 1276, 423);

/// "档案详情关闭" 按钮搜索区域（720p 基准 ltrb）
const CLOSE_BUTTON_ROI: Region2D<u32> = Region2D::from_ltrb(1180, 0, 1280, 100);

/// 模板匹配阈值
const THRESHOLD: f32 = 0.75;

/// 扫描当前子界面中的所有档案。
///
/// # 前置条件
/// - `session` 当前处于档案库子界面
///
/// # 工作流程
/// 1. 点击第 1 份档案 (401, 182) 进入档案详情页面
/// 2. 循环：OCR 标题 → 翻到下一篇 → 直到翻不动
/// 3. 点击关闭返回子界面
pub fn scan_current_sub_scene(session: &mut Session, scene_manager: &SceneManager) -> Result<()> {
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
        match session.ocr_in_roi(&screenshot, OCR_ROI) {
            Ok(text) if !text.trim().is_empty() => {
                success!("第 {} 份档案标题：{}", archive_count, text.trim());
            }
            Ok(_) => {
                success!("第 {} 份档案标题：（空）", archive_count);
            }
            Err(e) => {
                // OCR 失败不中断流程，记录日志继续
                debug!("OCR 识别失败（第 {archive_count} 份）: {e:#}");
                success!("第 {} 份档案标题：（OCR 识别失败）", archive_count);
            }
        }

        // 2b. 尝试翻到下一篇
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
