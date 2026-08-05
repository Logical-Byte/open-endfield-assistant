//! 档案库相关场景的识别实现。
//!
//! 每个场景实现 [`Scene`] trait，负责自我识别和定义跳转关系。
//!
//! 场景识别优先级（从具体到笼统，按此顺序注册到 SceneManager）：
//! 1. 档案详情页面（最具体）
//! 2. 档案库子界面
//! 3. 档案库主界面
//! 4. 协议终端
//! 5. 大世界
//! 6. 未知（兜底）

use std::sync::LazyLock;

use anyhow::Result;

use crate::{
    scene::{
        Scene, SceneAction, SceneId, SceneTransition, scene_manager::SceneManager, 档案库SubSceneId,
    },
    session::Session,
    utils::region::Region2D,
};

// ============================================================================
// 常用 ROI 和阈值常量（720p 基准）
// ============================================================================

/// 模板匹配默认阈值
const DEFAULT_THRESHOLD: f32 = 0.75;

/// 档案库标题 ROI
const ROI_档案库标题: Region2D<u32> = Region2D::from_ltrb(0, 0, 162, 76);

/// 右上角关闭按钮 ROI (1180, 0, 1280, 100)
const ROI_右上角关闭: Region2D<u32> = Region2D::from_ltrb(1180, 0, 1280, 100);

/// 水印 ROI (52, 482, 189, 618)
const ROI_水印: Region2D<u32> = Region2D::from_ltrb(52, 482, 189, 618);

/// 档案详情装饰 ROI (356, 34, 496, 77)
const ROI_档案详情装饰: Region2D<u32> = Region2D::from_ltrb(356, 34, 496, 77);

/// 协议终端按钮 ROI (1180, 0, 1280, 100)
const ROI_协议终端按钮: Region2D<u32> = Region2D::from_ltrb(1180, 0, 1280, 100);

/// 档案库按钮 ROI (971, 108, 1280, 700)
const ROI_档案库按钮: Region2D<u32> = Region2D::from_ltrb(971, 108, 1280, 700);

/// 音像存档按钮 ROI (692, 371, 959, 601)
const ROI_音像存档按钮: Region2D<u32> = Region2D::from_ltrb(692, 371, 959, 601);

/// 见闻辑录按钮 ROI (957, 135, 1221, 371)
const ROI_见闻辑录按钮: Region2D<u32> = Region2D::from_ltrb(957, 135, 1221, 371);

/// 中枢档案按钮 ROI (958, 369, 1220, 601)
const ROI_中枢档案按钮: Region2D<u32> = Region2D::from_ltrb(958, 369, 1220, 601);

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

    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>> {
        let screenshot = session.screencap_for_recognition()?;

        // 必须同时满足两个条件：有档案详情装饰 且 有关闭按钮
        let has_decoration = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/档案详情装饰.png",
                ROI_档案详情装饰,
                DEFAULT_THRESHOLD,
            )?
            .is_some();

        if !has_decoration {
            return Ok(None);
        }

        let has_close = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/档案详情关闭.png",
                ROI_右上角关闭,
                DEFAULT_THRESHOLD,
            )?
            .is_some();

        if has_close {
            Ok(Some(SceneId::档案详情页面))
        } else {
            Ok(None)
        }
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 从档案详情页面可以：
        // 1. 点击关闭按钮返回档案库子界面
        // 2. 点击"下一篇"进入下一份档案详情（由 scan_loop 处理，不在此定义）
        // 3. 点击"档案详情右箭头"进入下一份档案详情（同上）
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![SceneTransition {
                target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体), // 占位，实际在 scan_loop 中处理
                action: SceneAction::FindAndClickTemplate {
                    template_name: "情报档案库/档案详情关闭.png",
                    roi: ROI_右上角关闭,
                    threshold: DEFAULT_THRESHOLD,
                },
            }]
        });
        &T
    }
}

// ============================================================================
// 档案库子界面场景
// ============================================================================

/// 档案库子界面：音像存档/见闻辑录/中枢档案的子界面。
///
/// 识别特征：
/// - (0, 0, 162, 76) 范围内有 "情报档案库标题"
/// - (52, 482, 189, 618) 范围内有水印（音像/见闻/中枢）
/// - (1180, 0, 1280, 100) 范围内有 "档案库子界面关闭" 按钮
/// - 通过颜色 ROI 判断具体是哪个子界面
pub struct Scene档案库子界面;

impl Scene for Scene档案库子界面 {
    fn id(&self) -> SceneId {
        // 子界面返回一个通用的 ID，实际识别时 try_recognize 返回更具体的
        SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体)
    }

    fn name(&self) -> &'static str {
        "档案库子界面"
    }

    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>> {
        let screenshot = session.screencap_for_recognition()?;

        // 1. 检查标题
        let has_title = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/情报档案库标题.png",
                ROI_档案库标题,
                DEFAULT_THRESHOLD,
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查子界面关闭按钮（区别于主界面的关闭按钮）
        let has_close = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/档案库子界面关闭.png",
                ROI_右上角关闭,
                DEFAULT_THRESHOLD,
            )?
            .is_some();
        if !has_close {
            return Ok(None);
        }

        // 3. 判断属于哪个分类（通过水印）
        let category = self.detect_category(session, &screenshot)?;
        let Some(category) = category else {
            return Ok(None);
        };

        // 4. 判断具体是哪个子界面（通过颜色 ROI）
        let sub_scene = self.detect_sub_scene(session, category)?;
        Ok(Some(SceneId::档案库子界面(sub_scene)))
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 档案库子界面的跳转：
        // - 点击关闭 → 档案库主界面
        // - 点击侧边栏 tab → 同分类的其他子界面
        // - 点击第 1 份档案 (401, 182) → 档案详情页面
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![
                SceneTransition {
                    target: SceneId::档案库主界面,
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/档案库子界面关闭.png",
                        roi: ROI_右上角关闭,
                        threshold: DEFAULT_THRESHOLD,
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
    /// 通过水印判断属于哪个分类（音像存档/见闻辑录/中枢档案）。
    fn detect_category(
        &self,
        session: &mut Session,
        screenshot: &image::RgbaImage,
    ) -> Result<Option<&'static str>> {
        // 按顺序检测水印
        let categories = [
            ("音像存档", "情报档案库/音像存档水印.png"),
            ("见闻辑录", "情报档案库/见闻辑录水印.png"),
            ("中枢档案", "情报档案库/中枢档案水印.png"),
        ];

        for (name, template) in &categories {
            if session
                .find_template_in_roi(screenshot, template, ROI_水印, DEFAULT_THRESHOLD)?
                .is_some()
            {
                return Ok(Some(name));
            }
        }

        Ok(None)
    }

    /// 通过颜色 ROI 判断具体是哪个子界面。
    ///
    /// 颜色 ROI：从上到下 3 个，ltwh 分别为：
    /// - (180, 120, 60, 36) — tab 0
    /// - (180, 184, 60, 36) — tab 1
    /// - (180, 248, 60, 36) — tab 2
    ///
    /// 深色（灰度 < 128）表示当前在此子界面。
    fn detect_sub_scene(
        &self,
        session: &mut Session,
        category: &str,
    ) -> Result<档案库SubSceneId> {
        // 注意：需要重新截图（因为前面多用了 find_template_in_roi 可能消耗了截图）
        let screenshot = session.screencap_for_recognition()?;

        let tab0_dark = session.is_roi_dark_ltwh(&screenshot, 180, 120, 60, 36, DARK_THRESHOLD);
        let tab1_dark = session.is_roi_dark_ltwh(&screenshot, 180, 184, 60, 36, DARK_THRESHOLD);
        let tab2_dark = session.is_roi_dark_ltwh(&screenshot, 180, 248, 60, 36, DARK_THRESHOLD);

        match category {
            "音像存档" => {
                // 音像存档只有一个子界面，不需要判断颜色
                Ok(档案库SubSceneId::音像存档_多媒体)
            }
            "见闻辑录" => {
                // 见闻辑录有 3 个子界面，需要判断全部 3 个 roi
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
                // 中枢档案有 2 个子界面，只需要判断前 2 个 roi
                if tab0_dark {
                    Ok(档案库SubSceneId::中枢档案_中枢档案)
                } else if tab1_dark {
                    Ok(档案库SubSceneId::中枢档案_调查报告)
                } else {
                    // 默认当作中枢档案
                    Ok(档案库SubSceneId::中枢档案_中枢档案)
                }
            }
            _ => {
                // 无法识别的分类
                anyhow::bail!("未知的档案库子界面分类: {category}");
            }
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
/// - 能找到三个入口按钮（音像存档/见闻辑录/中枢档案）中的至少一个
pub struct Scene档案库主界面;

impl Scene for Scene档案库主界面 {
    fn id(&self) -> SceneId {
        SceneId::档案库主界面
    }

    fn name(&self) -> &'static str {
        "档案库主界面"
    }

    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>> {
        let screenshot = session.screencap_for_recognition()?;

        // 1. 检查标题
        let has_title = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/情报档案库标题.png",
                ROI_档案库标题,
                DEFAULT_THRESHOLD,
            )?
            .is_some();
        if !has_title {
            return Ok(None);
        }

        // 2. 检查主界面关闭按钮（与子界面关闭按钮不同）
        let has_main_close = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/档案库主界面关闭.png",
                ROI_右上角关闭,
                DEFAULT_THRESHOLD,
            )?
            .is_some();
        if !has_main_close {
            // 如果不是主界面关闭按钮，可能是子界面（有不同按钮）
            return Ok(None);
        }

        // 3. 确认至少有一个入口按钮存在
        let has_any_entry = session
            .find_template_in_roi(
                &screenshot,
                "情报档案库/音像存档.png",
                ROI_音像存档按钮,
                DEFAULT_THRESHOLD,
            )?
            .is_some()
            || session
                .find_template_in_roi(
                    &screenshot,
                    "情报档案库/见闻辑录.png",
                    ROI_见闻辑录按钮,
                    DEFAULT_THRESHOLD,
                )?
                .is_some()
            || session
                .find_template_in_roi(
                    &screenshot,
                    "情报档案库/中枢档案.png",
                    ROI_中枢档案按钮,
                    DEFAULT_THRESHOLD,
                )?
                .is_some();

        if has_any_entry {
            Ok(Some(SceneId::档案库主界面))
        } else {
            Ok(None)
        }
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
                        threshold: DEFAULT_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::音像存档_多媒体),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/音像存档.png",
                        roi: ROI_音像存档按钮,
                        threshold: DEFAULT_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::见闻辑录_纸质记录),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/见闻辑录.png",
                        roi: ROI_见闻辑录按钮,
                        threshold: DEFAULT_THRESHOLD,
                    },
                },
                SceneTransition {
                    target: SceneId::档案库子界面(档案库SubSceneId::中枢档案_中枢档案),
                    action: SceneAction::FindAndClickTemplate {
                        template_name: "情报档案库/中枢档案.png",
                        roi: ROI_中枢档案按钮,
                        threshold: DEFAULT_THRESHOLD,
                    },
                },
            ]
        });
        &T
    }
}

// ============================================================================
// 协议终端界面场景
// ============================================================================

/// 协议终端界面。
///
/// 识别特征：
/// - (971, 108, 1280, 700) 范围内有 "档案库" 按钮
pub struct Scene协议终端;

impl Scene for Scene协议终端 {
    fn id(&self) -> SceneId {
        SceneId::协议终端
    }

    fn name(&self) -> &'static str {
        "协议终端"
    }

    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>> {
        let screenshot = session.screencap_for_recognition()?;

        let found = session
            .find_template_in_roi(&screenshot, "档案库.png", ROI_档案库按钮, DEFAULT_THRESHOLD)?
            .is_some();

        if found {
            Ok(Some(SceneId::协议终端))
        } else {
            Ok(None)
        }
    }

    fn transitions(&self) -> &[SceneTransition] {
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![SceneTransition {
                target: SceneId::档案库主界面,
                action: SceneAction::FindAndClickTemplate {
                    template_name: "档案库.png",
                    roi: ROI_档案库按钮,
                    threshold: DEFAULT_THRESHOLD,
                },
            }]
        });
        &T
    }
}

// ============================================================================
// 大世界界面场景
// ============================================================================

/// 大世界界面。
///
/// 识别特征：
/// - (1180, 0, 1280, 100) 范围内有 "协议终端" 按钮
pub struct Scene大世界;

impl Scene for Scene大世界 {
    fn id(&self) -> SceneId {
        SceneId::大世界
    }

    fn name(&self) -> &'static str {
        "大世界"
    }

    fn try_recognize(&self, session: &mut Session) -> Result<Option<SceneId>> {
        let screenshot = session.screencap_for_recognition()?;

        let found = session
            .find_template_in_roi(
                &screenshot,
                "协议终端.png",
                ROI_协议终端按钮,
                DEFAULT_THRESHOLD,
            )?
            .is_some();

        if found {
            Ok(Some(SceneId::大世界))
        } else {
            Ok(None)
        }
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 从大世界按 ESC 键进入协议终端
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(|| {
            vec![SceneTransition {
                target: SceneId::协议终端,
                action: SceneAction::PressKey { vk_code: 0x1B }, // VK_ESCAPE
            }]
        });
        &T
    }
}

// ============================================================================
// 未知界面场景（兜底）
// ============================================================================

/// 未知界面：当所有场景都无法识别时使用。
/// 这只是一个占位，不允许在此场景执行导航。
pub struct Scene未知;

impl Scene for Scene未知 {
    fn id(&self) -> SceneId {
        SceneId::未知
    }

    fn name(&self) -> &'static str {
        "未知界面"
    }

    fn try_recognize(&self, _session: &mut Session) -> Result<Option<SceneId>> {
        // 未知场景总是返回自身（作为兜底）
        Ok(Some(SceneId::未知))
    }

    fn transitions(&self) -> &[SceneTransition] {
        // 未知场景无法跳转
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(Vec::new);
        &T
    }
}

// ============================================================================
// 辅助函数：创建 SceneManager 并注册所有场景
// ============================================================================

/// 创建并配置好所有档案库相关场景的 SceneManager。
///
/// 场景按从具体到笼统的顺序注册，确保更具体的场景优先被检测到。
pub fn create_scene_manager() -> SceneManager {
    let mut sm = SceneManager::new();

    // 注册顺序很重要：越具体的场景越先注册
    sm.register(Box::new(Scene档案详情页面)); // 1. 最具体
    sm.register(Box::new(Scene档案库子界面)); // 2. 子界面
    sm.register(Box::new(Scene档案库主界面)); // 3. 主界面
    sm.register(Box::new(Scene协议终端)); // 4. 协议终端
    sm.register(Box::new(Scene大世界)); // 5. 大世界
    sm.register(Box::new(Scene未知)); // 6. 兜底

    sm.build_navigation_graph();
    sm
}
