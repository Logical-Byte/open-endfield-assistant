//! 大世界场景：识别"协议终端"按钮，按 ESC 键进入协议终端。

use std::sync::LazyLock;

use anyhow::Result;

use super::super::model::{Scene, SceneAction, SceneId, SceneTransition};
use super::TEMPLATE_MATCH_THRESHOLD;
use crate::session::Session;
use crate::utils::region::Region2D;

/// 协议终端按钮 ROI (1180, 0, 1280, 100)
const ROI_协议终端按钮: Region2D<u32> = Region2D::from_ltrb(1180, 0, 1280, 100);

/// 大世界界面。
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
                TEMPLATE_MATCH_THRESHOLD,
            )?
            .is_some();

        Ok(if found {
            Some(SceneId::大世界)
        } else {
            None
        })
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
