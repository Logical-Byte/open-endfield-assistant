//! 场景 trait 与跳转描述。

use anyhow::Result;
use image::RgbaImage;

use crate::automation::TemplateMatching;

use super::SceneId;
use crate::scene::transition::Transition;

/// 场景 trait：每个游戏界面实现此 trait。
///
/// 每个场景负责：
/// 1. 识别自身（`try_recognize`）
/// 2. 定义可跳转的目标场景（`transitions`）
///
/// `Send + Sync`：场景实现均为零大小结构体，自动满足；
/// 同时允许 `Arc<SceneManager>` 跨线程共享（扫描线程 / 命令线程共用）。
pub trait Scene: Send + Sync {
    /// 返回此场景的唯一标识符。
    fn id(&self) -> SceneId;

    /// 返回场景名称（用于日志）。
    fn name(&self) -> &'static str;

    /// 尝试识别当前识别帧是否为此场景。
    ///
    /// # 参数
    /// - `screenshot`: 本轮场景分类共享的 720p 识别帧
    /// - `cx`: 工作流自动化能力，不暴露具体会话实现
    ///
    /// # 返回
    /// - `Ok(Some(scene_id))`: 识别成功，返回确切的场景 ID（子界面可能需要返回更具体的 ID）
    /// - `Ok(None)`: 不是此场景
    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<SceneId>>;

    /// 返回从此场景可以跳转到的目标场景及跳转方式。
    fn transitions(&self) -> &[Transition<'static>];
}
