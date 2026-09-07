//! 大世界场景：识别"协议终端"按钮，按 ESC 键进入协议终端。

use anyhow::Result;
use image::RgbaImage;

use super::super::{
    model::{Scene, SceneId},
    transition::{Op, Transition},
};
use super::TEMPLATE_MATCH_THRESHOLD;
use crate::{
    automation::{Key, TemplateMatching, TemplateTarget},
    utils::region::{Region2D, ltrb},
};

/// 协议终端按钮 ROI (1180, 0, 1280, 100)
const ROI_协议终端按钮: Region2D<u32> = ltrb!(1180, 0, 1280, 100);

/// 大世界界面。
pub struct Scene大世界;

impl Scene for Scene大世界 {
    fn id(&self) -> SceneId {
        SceneId::大世界
    }

    fn name(&self) -> &'static str {
        "大世界"
    }

    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        templates: &mut dyn TemplateMatching,
    ) -> Result<Option<SceneId>> {
        let found = templates
            .find_template(
                screenshot,
                &TemplateTarget {
                    template_name: "协议终端.png",
                    roi: ROI_协议终端按钮,
                    threshold: TEMPLATE_MATCH_THRESHOLD,
                },
            )?
            .is_some();

        Ok(if found {
            Some(SceneId::大世界)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[Transition<'static>] {
        // 从大世界按 ESC 键进入协议终端
        static TRANSITIONS: &[Transition<'static>] = &[Transition {
            target: SceneId::协议终端,
            ops: &[Op::PressKey(Key::Escape)],
        }];
        TRANSITIONS
    }
}
