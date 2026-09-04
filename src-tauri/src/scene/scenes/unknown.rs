//! Fallback scene used when no concrete recognizer matches.

use std::sync::LazyLock;

use anyhow::Result;

use super::super::model::{Scene, SceneId, SceneTransition};
use crate::session::RecognitionContext;

/// 未知界面（兜底）：所有场景都无法识别时使用；不允许从此场景导航。
pub struct Scene未知;

impl Scene for Scene未知 {
    fn id(&self) -> SceneId {
        SceneId::未知
    }

    fn name(&self) -> &'static str {
        "未知界面"
    }

    fn try_recognize(&self, _context: &mut RecognitionContext<'_>) -> Result<Option<SceneId>> {
        // 未知场景总是返回自身（作为兜底）
        Ok(Some(SceneId::未知))
    }

    fn transitions(&self) -> &[SceneTransition] {
        static T: LazyLock<Vec<SceneTransition>> = LazyLock::new(Vec::new);
        &T
    }
}
