use super::args::{Command, Rect};
use crate::automation::ScreenCapture;
use crate::{automation, navigation, vision::template_matching};
use anyhow::{Context, Result};
use clap::ValueEnum;
use image::RgbImage;
use serde_json::{Value, json};
use std::path::Path;

pub(super) struct Output {
    pub json: Value,
    pub human: String,
}

pub(super) struct Error {
    pub kind: &'static str,
    pub source: anyhow::Error,
}

impl From<anyhow::Error> for Error {
    fn from(source: anyhow::Error) -> Self {
        Self {
            kind: "runtime_error",
            source,
        }
    }
}

pub(super) fn execute(command: &Command) -> Result<Output, Error> {
    match command {
        Command::MatchImage {
            image,
            template,
            region,
        } => {
            let path = image.canonicalize().with_context(|| {
                format!("failed to resolve input image path: {}", image.display())
            })?;
            let image = image::open(&path)
                .with_context(|| format!("failed to decode input image: {}", path.display()))?
                .to_rgba8();
            let input =
                json!({"kind":"file", "path":path, "width":image.width(), "height":image.height()});
            match_image(command.name(), &image, input, template, *region).map_err(Error::from)
        }
        Command::Connect => {
            let session = connect()?;
            let (width, height) = session.client_size();
            Ok(Output {
                json: json!({"ok":true, "command":"connect", "client_size":{"width":width,"height":height}}),
                human: format!("Connected: client size {width}x{height}"),
            })
        }
        Command::Navigate { state } => {
            let mut session = connect()?;
            let target = state.ui_state();
            let state = state
                .to_possible_value()
                .expect("CLI state has a value")
                .get_name()
                .to_owned();
            navigation::Navigator::new()
                .navigate_to(target, &mut session)
                .with_context(|| format!("failed to navigate to {state}"))?;
            Ok(Output {
                json: json!({"ok":true,"command":"navigate","state":state}),
                human: format!("Navigated to {state}"),
            })
        }
        Command::Screenshot { output, crop } => {
            let mut session = connect()?;
            let (width, height) = session.client_size();
            let mut image = session.screenshot()?;
            let normalized_size = json!({"width":image.width(),"height":image.height()});
            if let Some(crop) = crop {
                crop.validate(image.width(), image.height())
                    .context("invalid --crop for normalized screenshot")?;
                image =
                    image::imageops::crop_imm(&image, crop.left, crop.top, crop.width, crop.height)
                        .to_image();
            }
            let path = super::screenshot::save(
                &image,
                output.as_deref(),
                &std::env::current_dir().context("read current directory")?,
            )?;
            Ok(Output {
                human: format!(
                    "Saved: {}\ncapture: {width}x{height}\nnormalized: {normalized_size}\ncrop: {}\nsaved: {}x{}",
                    path.display(),
                    serde_json::to_string(crop).expect("rectangle serializes"),
                    image.width(),
                    image.height()
                ),
                json: json!({"ok":true,"command":"screenshot","path":path,
                    "capture_size":{"width":width,"height":height},"normalized_size":normalized_size,
                    "crop":crop,"saved_size":{"width":image.width(),"height":image.height()}}),
            })
        }
        Command::Match { template, region } => {
            let mut session = connect()?;
            let image = session.screenshot()?;
            let input = json!({"kind":"game", "width":image.width(),"height":image.height()});
            match_image(command.name(), &image, input, template, *region).map_err(Error::from)
        }
    }
}

fn connect() -> Result<automation::Session, Error> {
    #[cfg(windows)]
    {
        use crate::{app_paths, platform, vision};
        use std::sync::{Arc, Mutex};
        platform::window::set_thread_dpi_awareness_context();
        let paths = app_paths::AppPaths::new().map_err(anyhow::Error::msg)?;
        let ocr = vision::ocr::OcrEngine::new(
            rapidocr_core::config::PipelineConfig::recognition_only(),
            &paths.models_dir(),
        )?;
        automation::Session::connect(&Arc::new(Mutex::new(ocr)), automation::new_stop_token())
            .map_err(Error::from)
    }
    #[cfg(not(windows))]
    {
        Err(Error {
            kind: "platform_unsupported",
            source: anyhow::anyhow!(
                "game window commands are unsupported on {}",
                std::env::consts::OS
            ),
        })
    }
}

/// One explicit filesystem image, owned for the duration of a single search.
struct FileTemplate(RgbImage);
impl template_matching::TemplateProvider for FileTemplate {
    fn get(&mut self, _name: &str) -> Result<&RgbImage> {
        Ok(&self.0)
    }
}

fn match_image(
    command: &str,
    image: &image::RgbaImage,
    input: Value,
    template: &Path,
    region: Rect,
) -> Result<Output> {
    let search = region
        .validate(image.width(), image.height())
        .context("invalid --region for matching input")?;
    let template_path = template
        .canonicalize()
        .with_context(|| format!("failed to resolve template path: {}", template.display()))?;
    let mut provider = FileTemplate(
        image::open(&template_path)
            .with_context(|| {
                format!(
                    "failed to decode template image: {}",
                    template_path.display()
                )
            })?
            .to_rgb8(),
    );
    let template =
        json!({"path":template_path, "width":provider.0.width(), "height":provider.0.height()});
    let matched = template_matching::find(image, "explicit-file", search, &mut provider)
        .with_context(|| {
            format!(
                "failed to match template {} ({}x{}) in region ({region}) of {}x{} input",
                template_path.display(),
                provider.0.width(),
                provider.0.height(),
                image.width(),
                image.height()
            )
        })?;
    let matched_region = Rect::from_region(matched.region);
    Ok(Output {
        human: format!(
            "input: {input}\ntemplate: {template}\nsearch region: {},{},{},{}\nmatch region: {},{},{},{}\nscore: {:.6}",
            region.left,
            region.top,
            region.width,
            region.height,
            matched_region.left,
            matched_region.top,
            matched_region.width,
            matched_region.height,
            matched.score
        ),
        json: json!({"ok":true, "command":command, "input":input, "template":template,
            "search_region":region, "match":{"region":matched_region,"score":matched.score}}),
    })
}
