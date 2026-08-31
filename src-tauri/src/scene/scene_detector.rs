use std::{
    collections::HashMap,
    mem::{Discriminant, discriminant},
};

use anyhow::{Context, Result, bail};
use tracing::{debug, warn};

use super::{Scene, SceneId};
use crate::session::Session;

/// 已注册场景的查找与检测入口。
pub(super) struct SceneDetector {
    scenes: Vec<Box<dyn Scene>>,
    scene_index: HashMap<Discriminant<SceneId>, usize>,
}

impl SceneDetector {
    pub(super) fn new(scenes: Vec<Box<dyn Scene>>) -> Self {
        let mut scene_index = HashMap::new();
        for (index, scene) in scenes.iter().enumerate() {
            let id = scene.id();
            assert!(
                scene_index.insert(discriminant(&id), index).is_none(),
                "同一 SceneId 变体重复注册: {:?}",
                id
            );
        }

        Self {
            scenes,
            scene_index,
        }
    }

    pub(super) fn detect_current_scene(&self, session: &mut Session) -> Result<SceneId> {
        // Preserve the detection contract: capture failures must abort even when
        // the fallback unknown scene would otherwise match.
        session.screencap_for_recognition()?;

        for scene in &self.scenes {
            // 模板或图像错误必须终止任务，不能降级成「未知场景」。
            let id = scene
                .try_recognize(session)
                .with_context(|| format!("场景识别出错 ({})", scene.name()))?;
            if let Some(id) = id {
                debug!("场景检测: 当前处于 {:?}", id);
                return Ok(id);
            }
        }

        warn!("场景检测: 当前场景无法识别，返回「未知」");
        Ok(SceneId::未知)
    }

    /// 仅使用期望场景的识别器检查当前场景。
    pub(super) fn recognizes_scene(
        &self,
        expected: SceneId,
        session: &mut Session,
    ) -> Result<bool> {
        if expected == SceneId::未知 {
            bail!("未知场景不能作为预期场景");
        }

        let scene = self
            .get_registered_scene(expected)
            .ok_or_else(|| anyhow::anyhow!("未注册的场景: {:?}", expected))?;
        let recognized = scene
            .try_recognize(session)
            .with_context(|| format!("场景识别出错 ({})", scene.name()))?;
        Ok(recognized == Some(expected))
    }

    pub(super) fn get_registered_scene(&self, hint: SceneId) -> Option<&dyn Scene> {
        let index = self.scene_index.get(&discriminant(&hint))?;
        Some(self.scenes[*index].as_ref())
    }
}
