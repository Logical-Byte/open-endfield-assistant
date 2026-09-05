//! 协议终端场景：识别"档案库"按钮，点击进入档案库主界面。

use anyhow::Result;
use image::RgbaImage;

use super::super::{
    model::{Scene, SceneId},
    transition::{Op, Transition},
};
use super::TEMPLATE_MATCH_THRESHOLD;
use crate::{
    automation::{Automation, TemplateTarget},
    utils::region::{Region2D, ltrb},
};

/// 档案库按钮 ROI (971, 108, 1280, 700)
const ROI_档案库按钮: Region2D<u32> = ltrb!(971, 108, 1280, 700);

/// 协议终端界面。
pub struct Scene协议终端;

impl Scene for Scene协议终端 {
    fn id(&self) -> SceneId {
        SceneId::协议终端
    }

    fn name(&self) -> &'static str {
        "协议终端"
    }

    fn try_recognize(
        &self,
        screenshot: &RgbaImage,
        cx: &mut dyn Automation,
    ) -> Result<Option<SceneId>> {
        let found = cx
            .find_template(
                screenshot,
                &TemplateTarget {
                    template_name: "档案库.png",
                    roi: ROI_档案库按钮,
                    threshold: TEMPLATE_MATCH_THRESHOLD,
                },
            )?
            .is_some();

        Ok(if found {
            Some(SceneId::协议终端)
        } else {
            None
        })
    }

    fn transitions(&self) -> &[Transition<'static>] {
        static TRANSITIONS: &[Transition<'static>] = &[Transition {
            target: SceneId::档案库主界面,
            ops: &[Op::FindAndClickTemplate(TemplateTarget {
                template_name: "档案库.png",
                roi: ROI_档案库按钮,
                threshold: TEMPLATE_MATCH_THRESHOLD,
            })],
        }];
        TRANSITIONS
    }
}
