//! 扫描计划：定义 6 个子分类的遍历顺序。

use crate::scene::档案库SubSceneId;

/// 一个扫描步骤：从档案库主界面点击哪个按钮进入哪个子分类，
/// 该分类下有哪些子界面需要扫描。
pub struct ScanStep {
    /// 从档案库主界面点击此模板进入该子分类
    pub entry_template: &'static str,
    /// 进入后的初始子界面
    pub first_sub_scene: 档案库SubSceneId,
    /// 该分类下需要扫描的所有子界面（按 tab 顺序排列）
    /// 音像存档: [多媒体]；见闻辑录: [纸质记录, 电子档案, 藏品]；中枢档案: [中枢档案, 调查报告]
    pub sub_scenes: &'static [档案库SubSceneId],
}

/// 6 个子分类的完整扫描计划。
pub const SCAN_PLAN: &[ScanStep] = &[
    ScanStep {
        // 音像存档 — 只有 1 个子界面
        entry_template: "情报档案库/音像存档.png",
        first_sub_scene: 档案库SubSceneId::音像存档_多媒体,
        sub_scenes: &[档案库SubSceneId::音像存档_多媒体],
    },
    ScanStep {
        // 见闻辑录 — 有 3 个子界面（纸质记录、电子档案、藏品）
        entry_template: "情报档案库/见闻辑录.png",
        first_sub_scene: 档案库SubSceneId::见闻辑录_纸质记录,
        sub_scenes: &[
            档案库SubSceneId::见闻辑录_纸质记录,
            档案库SubSceneId::见闻辑录_电子档案,
            档案库SubSceneId::见闻辑录_藏品,
        ],
    },
    ScanStep {
        // 中枢档案 — 有 2 个子界面（中枢档案、调查报告）
        entry_template: "情报档案库/中枢档案.png",
        first_sub_scene: 档案库SubSceneId::中枢档案_中枢档案,
        sub_scenes: &[
            档案库SubSceneId::中枢档案_中枢档案,
            档案库SubSceneId::中枢档案_调查报告,
        ],
    },
];

/// 子界面所属的档案库分类名称（扫描结果卡片的 category 字段）。
pub fn category_of(sub_scene: 档案库SubSceneId) -> &'static str {
    match sub_scene {
        档案库SubSceneId::音像存档_多媒体 => "音像存档",
        档案库SubSceneId::见闻辑录_纸质记录
        | 档案库SubSceneId::见闻辑录_电子档案
        | 档案库SubSceneId::见闻辑录_藏品 => "见闻辑录",
        档案库SubSceneId::中枢档案_中枢档案 | 档案库SubSceneId::中枢档案_调查报告 => {
            "中枢档案"
        }
    }
}
