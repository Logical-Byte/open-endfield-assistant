//! 档案库相关场景：主界面 / 子界面 / 详情页面。
//!
//! 识别优先级（具体 → 笼统）由 `SceneManager::new` 的注册顺序决定，
//! 本模块内部三个场景的相对顺序：档案详情页面 > 档案库子界面 > 档案库主界面。

use std::sync::LazyLock;

use anyhow::Result;

use super::super::model::{Scene, SceneAction, SceneId, SceneTransition, 档案库SubSceneId};
use super::TEMPLATE_MATCH_THRESHOLD;
use crate::session::RecognitionContext;
use crate::utils::region::{Region2D, ltrb, ltwh};

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
const ROI_音像存档按钮: Region2D<u32> = ltrb!(692, 371, 959, 601);

/// 见闻辑录按钮 ROI (957, 135, 1221, 371)
const ROI_见闻辑录按钮: Region2D<u32> = ltrb!(957, 135, 1221, 371);

/// 中枢档案按钮 ROI (958, 369, 1220, 601)
const ROI_中枢档案按钮: Region2D<u32> = ltrb!(958, 369, 1220, 601);

/// 颜色判断阈值：灰度 < 128 视为深色（选中状态）
const DARK_THRESHOLD: u8 = 128;

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

    fn try_recognize(&self, context: &mut RecognitionContext<'_>) -> Result<Option<SceneId>> {
        // 必须同时满足两个条件：有档案详情装饰 且 有关闭按钮
        let has_decoration = context
            .find_template_in_roi(
                "情报档案库/档案详情装饰.png",
                ROI_档案详情装饰,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();
        if !has_decoration {
            return Ok(None);
        }

        let has_close = context
            .find_template_in_roi(
                "情报档案库/档案详情关闭.png",
                ROI_右上角关闭,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();

        Ok(if has_close {
            Some(SceneId::档案详情页面)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 点击关闭按钮返回档案库子界面。
        // （"下一篇"/右箭头由 scan_loop 直接处理，不在此定义。）
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![SceneTransition {
                target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体), // 占位，实际返回任意子界面
                action: SceneAction::FindAndClickTemplate {
                    template_name: "情报档案库/档案详情关闭.png",
                    roi: ROI_右上角关闭,
                    threshold: TEMPLATE_MATCH_THRESHOLD,
                },
            }]
        });
        &T
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

    fn try_recognize(&self, context: &mut RecognitionContext<'_>) -> Result<Option<SceneId>> {
        // 1. 检查标题
        let has_title = context
            .find_template_in_roi(
                "情报档案库/情报档案库标题.png",
                ROI_档案库标题,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查子界面关闭按钮（区别于主界面的关闭按钮）
        let has_close = context
            .find_template_in_roi(
                "情报档案库/档案库子界面关闭.png",
                ROI_右上角关闭,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();
        if !has_close {
            return Ok(None);
        }

        // 3. 判断属于哪个分类（通过水印）
        let Some(category) = self.detect_category(context)? else {
            return Ok(None);
        };

        // 4. 判断具体是哪个子界面（通过 tab 颜色 ROI）
        let sub_scene = self.detect_sub_scene(context, category)?;
        Ok(Some(SceneId::档案库子界面(sub_scene)))
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 档案库子界面的跳转：
        // - 点击关闭 → 档案库主界面
        // - 点击第 1 份档案 (401, 182) → 档案详情页面
        // - 点击侧边栏 tab → 同分类的其他子界面（由任务层用 ClickSubTab 处理）
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![
                SceneTransition {
                    target: SceneId::档案库主界面,
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/档案库子界面关闭.png",
                        roi: ROI_右上角关闭,
                        threshold: TEMPLATE_MATCH_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案详情页面,
                    action: SceneAction::ClickAt { x: 401, y: 182 },
                },
            ]
        });
        &T
    }
}

impl Scene档案库子界面 {
    /// 通过水印判断属于哪个分类（音像存档 / 见闻辑录 / 中枢档案）。
    fn detect_category(
        &self,
        context: &mut RecognitionContext<'_>,
    ) -> Result<Option<&'static str>> {
        let categories = [
            ("音像存档", "情报档案库/音像存档水印.png"),
            ("见闻辑录", "情报档案库/见闻辑录水印.png"),
            ("中枢档案", "情报档案库/中枢档案水印.png"),
        ];

        for (name, template) in &categories {
            if context
                .find_template_in_roi(template, ROI_水印, TEMPLATE_MATCH_THRESHOLD)?
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
        context: &RecognitionContext<'_>,
        category: &str,
    ) -> Result<档案库SubSceneId> {
        let tab0_dark = context.is_roi_dark(ltwh!(180, 120, 60, 36), DARK_THRESHOLD);
        let tab1_dark = context.is_roi_dark(ltwh!(180, 184, 60, 36), DARK_THRESHOLD);
        let tab2_dark = context.is_roi_dark(ltwh!(180, 248, 60, 36), DARK_THRESHOLD);

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

    fn try_recognize(&self, context: &mut RecognitionContext<'_>) -> Result<Option<SceneId>> {
        // 1. 检查标题
        let has_title = context
            .find_template_in_roi(
                "情报档案库/情报档案库标题.png",
                ROI_档案库标题,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查主界面关闭按钮（与子界面关闭按钮不同）
        let has_main_close = context
            .find_template_in_roi(
                "情报档案库/档案库主界面关闭.png",
                ROI_右上角关闭,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();
        if !has_main_close {
            // 如果不是主界面关闭按钮，可能是子界面（按钮不同）
            return Ok(None);
        }

        // 3. 确认至少有一个入口按钮存在
        let has_any_entry = context
            .find_template_in_roi(
                "情报档案库/音像存档.png",
                ROI_音像存档按钮,
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some()
            || context
                .find_template_in_roi(
                    "情报档案库/见闻辑录.png",
                    ROI_见闻辑录按钮,
                    TEMPLATE_MATCH_THRESHOLD,
                )?
                .is_some()
            || context
                .find_template_in_roi(
                    "情报档案库/中枢档案.png",
                    ROI_中枢档案按钮,
                    TEMPLATE_MATCH_THRESHOLD,
                )?
                .is_some();

        Ok(if has_any_entry {
            Some(SceneId::档案库主界面)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 从档案库主界面可以：
        // - 点击主界面关闭 → 协议终端
        // - 点击音像存档 → 音像存档-多媒体子界面
        // - 点击见闻辑录 → 见闻辑录-纸质记录子界面
        // - 点击中枢档案 → 中枢档案-中枢档案子界面
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![
                SceneTransition {
                    target: SceneId::协议终端,
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/档案库主界面关闭.png",
                        roi: ROI_右上角关闭,
                        threshold: TEMPLATE_MATCH_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/音像存档.png",
                        roi: ROI_音像存档按钮,
                        threshold: TEMPLATE_MATCH_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_纸质记录),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/见闻辑录.png",
                        roi: ROI_见闻辑录按钮,
                        threshold: TEMPLATE_MATCH_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::中枢档案_中枢档案),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/中枢档案.png",
                        roi: ROI_中枢档案按钮,
                        threshold: TEMPLATE_MATCH_THRESHOLD,
                    },
                },
            ]
        });
        &T
    }
}
