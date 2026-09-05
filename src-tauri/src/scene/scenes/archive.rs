//! 档案库相关场景：主界面 / 子界面 / 详情页面。
//!
//! 识别优先级（具体 → 笼统）由 `SceneManager::new` 的注册顺序决定，
//! 本模块内部三个场景的相对顺序：档案详情页面 > 档案库子界面 > 档案库主界面。

use anyhow::Result;
use image::{RgbaImage, imageops};

use super::super::{
    model::{Scene, SceneId, 档案库SubSceneId},
    transition::{Op, Transition},
};
use super::TEMPLATE_MATCH_THRESHOLD;
use crate::{
    automation::{Point720p, TemplateMatching, TemplateTarget},
    utils::region::{Region2D, ltrb, ltwh},
};

// ============================================================================
// 常用 ROI 和阈值常量（720p 基准）
// ============================================================================

/// 档案库标题 ROI
const ROI_档案库标题: Region2D<u32> = ltrb!(0, 0, 162, 76);

/// 右上角关闭按钮 ROI (1180, 0, 1280, 100)
const ROI_右上角关闭: Region2D<u32> = ltrb!(1180, 0, 1280, 100);

/// 水印 ROI (52, 482, 189, 618)
const ROI_水印: Region2D<u32> = ltrb!(52, 482, 189, 618);

/// 档案详情装饰 ROI (356, 34, 496, 77)
const ROI_档案详情装饰: Region2D<u32> = ltrb!(356, 34, 496, 77);

/// 音像存档按钮 ROI (692, 371, 959, 601)
pub(crate) const ROI_音像存档按钮: Region2D<u32> = ltrb!(692, 371, 959, 601);

/// 见闻辑录按钮 ROI (957, 135, 1221, 371)
pub(crate) const ROI_见闻辑录按钮: Region2D<u32> = ltrb!(957, 135, 1221, 371);

/// 中枢档案按钮 ROI (958, 369, 1220, 601)
pub(crate) const ROI_中枢档案按钮: Region2D<u32> = ltrb!(958, 369, 1220, 601);

/// 档案库侧边栏从上到下三个 tab 的颜色检测与点击区域（720p 基准 LTWH）。
const TAB_CENTERS: [Point720p; 3] = [
    Point720p { x: 210, y: 138 },
    Point720p { x: 210, y: 202 },
    Point720p { x: 210, y: 266 },
];

pub(crate) const TAB_ROIS: [Region2D<u32>; 3] = [
    tab_roi(TAB_CENTERS[0]),
    tab_roi(TAB_CENTERS[1]),
    tab_roi(TAB_CENTERS[2]),
];

const fn tab_roi(center: Point720p) -> Region2D<u32> {
    ltwh!(center.x - 30, center.y - 18, 60, 36)
}

static SIDEBAR_TRANSITIONS: &[Transition<'static>] = &[
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体),
        ops: &[Op::Click(TAB_CENTERS[0])],
    },
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_纸质记录),
        ops: &[Op::Click(TAB_CENTERS[0])],
    },
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_电子档案),
        ops: &[Op::Click(TAB_CENTERS[1])],
    },
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_藏品),
        ops: &[Op::Click(TAB_CENTERS[2])],
    },
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::中枢档案_中枢档案),
        ops: &[Op::Click(TAB_CENTERS[0])],
    },
    Transition {
        target: SceneId::档案库子界面(档案库SubSceneId::中枢档案_调查报告),
        ops: &[Op::Click(TAB_CENTERS[1])],
    },
];

/// 返回同一档案分类中切换到目标子界面的侧边栏跳转。
pub(crate) fn sidebar_transition(target: 档案库SubSceneId) -> &'static Transition<'static> {
    SIDEBAR_TRANSITIONS
        .iter()
        .find(|transition| transition.target == SceneId::档案库子界面(target))
        .expect("每个档案库子界面都应声明侧边栏跳转")
}

/// 颜色判断阈值：灰度 < 128 视为深色（选中状态）
const DARK_THRESHOLD: u8 = 128;

const fn target(template_name: &'static str, roi: Region2D<u32>) -> TemplateTarget {
    TemplateTarget {
        template_name,
        roi,
        threshold: TEMPLATE_MATCH_THRESHOLD,
    }
}

fn is_roi_dark(screenshot: &RgbaImage, roi: Region2D<u32>, threshold: u8) -> bool {
    let cropped = imageops::crop_imm(screenshot, roi.x0(), roi.y0(), roi.width(), roi.height());
    let gray = imageops::grayscale(&cropped.to_image());
    let pixel_count = (roi.width() * roi.height()) as u64;
    if pixel_count == 0 {
        return false;
    }

    let total: u64 = gray.pixels().map(|pixel| pixel.0[0] as u64).sum();
    ((total / pixel_count) as u8) < threshold
}

// ============================================================================
// 档案详情页面场景
// ============================================================================

/// 档案详情页面：展示单份档案的完整内容。
///
/// 识别特征：
/// - (356, 34, 496, 77) 范围内有 "档案详情装饰"
/// - (1180, 0, 1280, 100) 范围内有 "档案详情关闭" 按钮
pub struct Scene档案详情页面;

impl Scene for Scene档案详情页面 {
    fn id(&self) -> SceneId {
        SceneId::档案详情页面
    }

    fn name(&self) -> &'static str {
        "档案详情页面"
    }

    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<SceneId>> {
        // 必须同时满足两个条件：有档案详情装饰 且 有关闭按钮
        let has_decoration = templates
            .find_template(
                screenshot,
                &target("情报档案库/档案详情装饰.png", ROI_档案详情装饰),
            )?
            .is_some();
        if !has_decoration {
            return Ok(None);
        }

        let has_close = templates
            .find_template(
                screenshot,
                &target("情报档案库/档案详情关闭.png", ROI_右上角关闭),
            )?
            .is_some();

        Ok(if has_close {
            Some(SceneId::档案详情页面)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[Transition<'static>] {
        // 点击关闭按钮返回档案库子界面。
        // （"下一篇"/右箭头由 scan_loop 直接处理，不在此定义。）
        static TRANSITIONS: &[Transition<'static>] = &[Transition {
            target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体), // 占位，实际返回任意子界面
            ops: &[Op::FindAndClickTemplate(target(
                "情报档案库/档案详情关闭.png",
                ROI_右上角关闭,
            ))],
        }];
        TRANSITIONS
    }
}

// ============================================================================
// 档案库子界面场景
// ============================================================================

/// 档案库子界面：音像存档 / 见闻辑录 / 中枢档案 的子界面。
///
/// 识别特征：
/// - (0, 0, 162, 76) 范围内有 "情报档案库标题"
/// - (52, 482, 189, 618) 范围内有水印（区分三大分类）
/// - (1180, 0, 1280, 100) 范围内有 "档案库子界面关闭" 按钮
/// - 通过侧边栏 tab 的颜色 ROI 判断具体是哪个子界面
pub struct Scene档案库子界面;

impl Scene for Scene档案库子界面 {
    fn id(&self) -> SceneId {
        // 导航图只注册一个"子界面"节点（子界面变体之间互达），故统一返回此 ID
        SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体)
    }

    fn name(&self) -> &'static str {
        "档案库子界面"
    }

    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<SceneId>> {
        // 1. 检查标题
        let has_title = templates
            .find_template(
                screenshot,
                &target("情报档案库/情报档案库标题.png", ROI_档案库标题),
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查子界面关闭按钮（区别于主界面的关闭按钮）
        let has_close = templates
            .find_template(
                screenshot,
                &target("情报档案库/档案库子界面关闭.png", ROI_右上角关闭),
            )?
            .is_some();
        if !has_close {
            return Ok(None);
        }

        // 3. 判断属于哪个分类（通过水印）
        let Some(category) = self.detect_category(screenshot, templates)? else {
            return Ok(None);
        };

        // 4. 判断具体是哪个子界面（通过 tab 颜色 ROI）
        let sub_scene = self.detect_sub_scene(screenshot, category)?;
        Ok(Some(SceneId::档案库子界面(sub_scene)))
    }

    fn transitions(&self) -> &[Transition<'static>] {
        // 档案库子界面的跳转：
        // - 点击关闭 → 档案库主界面
        // - 点击第 1 份档案 (401, 182) → 档案详情页面
        // - 点击侧边栏 tab → 同分类的其他子界面（由任务层直接处理业务顺序）
        static TRANSITIONS: &[Transition<'static>] = &[
            Transition {
                target: SceneId::档案库主界面,
                ops: &[Op::FindAndClickTemplate(target(
                    "情报档案库/档案库子界面关闭.png",
                    ROI_右上角关闭,
                ))],
            },
            Transition {
                target: SceneId::档案详情页面,
                ops: &[Op::Click(Point720p { x: 401, y: 182 })],
            },
        ];
        TRANSITIONS
    }
}

impl Scene档案库子界面 {
    /// 通过水印判断属于哪个分类（音像存档 / 见闻辑录 / 中枢档案）。
    fn detect_category(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<&'static str>> {
        let categories = [
            ("音像存档", "情报档案库/音像存档水印.png"),
            ("见闻辑录", "情报档案库/见闻辑录水印.png"),
            ("中枢档案", "情报档案库/中枢档案水印.png"),
        ];

        for (name, template) in &categories {
            if templates
                .find_template(screenshot, &target(template, ROI_水印))?
                .is_some()
            {
                return Ok(Some(name));
            }
        }

        Ok(None)
    }

    /// 通过 tab 颜色 ROI 判断具体是哪个子界面。
    ///
    /// 颜色 ROI 从上到下：ltwh 为 (180, 120, 60, 36)、(180, 184, 60, 36)、(180, 248, 60, 36)；
    /// 深色（灰度 < 阈值）表示当前选中该 tab。
    fn detect_sub_scene(
        &self,
        screenshot: &RgbaImage,
        category: &str,
    ) -> Result<档案库SubSceneId> {
        let tab0_dark = is_roi_dark(screenshot, TAB_ROIS[0], DARK_THRESHOLD);
        let tab1_dark = is_roi_dark(screenshot, TAB_ROIS[1], DARK_THRESHOLD);
        let tab2_dark = is_roi_dark(screenshot, TAB_ROIS[2], DARK_THRESHOLD);

        match category {
            "音像存档" => {
                // 音像存档只有一个子界面，无需判断颜色
                Ok(档案库SubSceneId::音像存档_多媒体)
            }
            "见闻辑录" => {
                if tab0_dark {
                    Ok(档案库SubSceneId::见闻辑录_纸质记录)
                } else if tab1_dark {
                    Ok(档案库SubSceneId::见闻辑录_电子档案)
                } else if tab2_dark {
                    Ok(档案库SubSceneId::见闻辑录_藏品)
                } else {
                    // 默认当作纸质记录（刚进入时的初始状态）
                    Ok(档案库SubSceneId::见闻辑录_纸质记录)
                }
            }
            "中枢档案" => {
                if tab0_dark {
                    Ok(档案库SubSceneId::中枢档案_中枢档案)
                } else if tab1_dark {
                    Ok(档案库SubSceneId::中枢档案_调查报告)
                } else {
                    // 默认当作中枢档案
                    Ok(档案库SubSceneId::中枢档案_中枢档案)
                }
            }
            _ => anyhow::bail!("未知的档案库子界面分类: {category}"),
        }
    }
}

// ============================================================================
// 档案库主界面场景
// ============================================================================

/// 档案库主界面：显示音像存档、见闻辑录、中枢档案三个入口。
///
/// 识别特征：
/// - (0, 0, 162, 76) 范围内有 "情报档案库标题"
/// - (1180, 0, 1280, 100) 范围内有 "档案库主界面关闭" 按钮
/// - 能找到三个入口按钮中的至少一个
pub struct Scene档案库主界面;

impl Scene for Scene档案库主界面 {
    fn id(&self) -> SceneId {
        SceneId::档案库主界面
    }

    fn name(&self) -> &'static str {
        "档案库主界面"
    }

    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<SceneId>> {
        // 1. 检查标题
        let has_title = templates
            .find_template(
                screenshot,
                &target("情报档案库/情报档案库标题.png", ROI_档案库标题),
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查主界面关闭按钮（与子界面关闭按钮不同）
        let has_main_close = templates
            .find_template(
                screenshot,
                &target("情报档案库/档案库主界面关闭.png", ROI_右上角关闭),
            )?
            .is_some();
        if !has_main_close {
            // 如果不是主界面关闭按钮，可能是子界面（按钮不同）
            return Ok(None);
        }

        // 3. 确认至少有一个入口按钮存在
        let has_any_entry = templates
            .find_template(
                screenshot,
                &target("情报档案库/音像存档.png", ROI_音像存档按钮),
            )?
            .is_some()
            || templates
                .find_template(
                    screenshot,
                    &target("情报档案库/见闻辑录.png", ROI_见闻辑录按钮),
                )?
                .is_some()
            || templates
                .find_template(
                    screenshot,
                    &target("情报档案库/中枢档案.png", ROI_中枢档案按钮),
                )?
                .is_some();

        Ok(if has_any_entry {
            Some(SceneId::档案库主界面)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[Transition<'static>] {
        // 从档案库主界面可以：
        // - 点击主界面关闭 → 协议终端
        // - 点击音像存档 → 音像存档-多媒体子界面
        // - 点击见闻辑录 → 见闻辑录-纸质记录子界面
        // - 点击中枢档案 → 中枢档案-中枢档案子界面
        static TRANSITIONS: &[Transition<'static>] = &[
            Transition {
                target: SceneId::协议终端,
                ops: &[Op::FindAndClickTemplate(target(
                    "情报档案库/档案库主界面关闭.png",
                    ROI_右上角关闭,
                ))],
            },
            Transition {
                target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体),
                ops: &[Op::FindAndClickTemplate(target(
                    "情报档案库/音像存档.png",
                    ROI_音像存档按钮,
                ))],
            },
            Transition {
                target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_纸质记录),
                ops: &[Op::FindAndClickTemplate(target(
                    "情报档案库/见闻辑录.png",
                    ROI_见闻辑录按钮,
                ))],
            },
            Transition {
                target: SceneId::档案库子界面(档案库SubSceneId::中枢档案_中枢档案),
                ops: &[Op::FindAndClickTemplate(target(
                    "情报档案库/中枢档案.png",
                    ROI_中枢档案按钮,
                ))],
            },
        ];
        TRANSITIONS
    }
}
