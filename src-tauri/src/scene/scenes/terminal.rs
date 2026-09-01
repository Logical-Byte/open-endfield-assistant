//! 协议终端场景：识别"档案库"按钮，点击进入档案库主界面。

use std::sync::LazyLock;

use anyhow::Result;

use super::super::{DEFAULT_THRESHOLD, Scene, SceneAction, SceneId, SceneTransition};
use crate::session::Session;
use crate::utils::region::Region2D;

/// 档案库按钮 ROI (971, 108, 1280, 700)
const ROI_档案库按钮: Region2D<u32> = Region2D::from_ltrb(971, 108, 1280, 700);

/// 协议终端界面。
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

        Ok(if found {
            Some(SceneId::协议终端)
        } else {
            None
        })
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
