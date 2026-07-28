use anyhow::Result;

use crate::{input::InputBase, screencap::ScreencapBase, template_matching::TemplateManager};

pub struct Navigator<I, S>
where
    I: InputBase,
    S: ScreencapBase,
{
    input: I,
    screencap: S,
    template_manager: TemplateManager,
}

impl<I, S> Navigator<I, S>
where
    I: InputBase,
    S: ScreencapBase,
{
    pub fn new(input: I, screencap: S, template_manager: TemplateManager) -> Self {
        Self {
            input,
            screencap,
            template_manager,
        }
    }

    pub fn navigate_to_档案库(&mut self) -> Result<()> {
        Ok(())
    }
}
