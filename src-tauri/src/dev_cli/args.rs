use crate::{navigation, utils::region::Region2D};
use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::{fmt, path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
#[command(
    name = "OEA dev",
    bin_name = "OEA dev",
    version = application_version(),
    about = "Developer tools (unstable interface)"
)]
pub(super) struct Dev {
    /// Emit one JSON result on stdout, or a runtime error on stderr
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

fn application_version() -> &'static str {
    #[derive(serde::Deserialize)]
    struct AppVersion<'a> {
        version: &'a str,
    }

    let config: AppVersion<'static> = serde_json::from_str(include_str!("../../tauri.conf.json"))
        .expect("tauri.conf.json must contain an application version");
    config.version
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Probe the production game session without capturing or sending input
    Connect,
    /// Navigate using production recognition, input, waits and retries
    Navigate { state: State },
    /// Capture and normalize to 1280x720 with production Lanczos3, then optionally crop
    #[command(
        after_help = "Output defaults to the current directory. Existing directories get a timestamped PNG.\nNew file paths require an existing parent and either .png or no extension. Never overwrites."
    )]
    Screenshot {
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Crop in normalized 1280x720 pixels; positive size, fully inside the image
        #[arg(long, value_name = "LEFT,TOP,WIDTH,HEIGHT")]
        crop: Option<Rect>,
    },
    /// Match a filesystem template against the production 1280x720 game capture
    Match {
        /// Template file, absolute or relative to the current directory
        #[arg(long, value_name = "PATH")]
        template: PathBuf,
        /// Search in 1280x720 pixels; positive size, fully inside the image
        #[arg(long, value_name = "LEFT,TOP,WIDTH,HEIGHT")]
        region: Rect,
    },
    /// Match local images at their original size, without connecting to a game
    MatchImage {
        /// Input image, absolute or relative to the current directory; never scaled
        #[arg(long, value_name = "PATH")]
        image: PathBuf,
        /// Template file, absolute or relative to the current directory
        #[arg(long, value_name = "PATH")]
        template: PathBuf,
        /// Search in original image pixels; positive size, fully inside the image
        #[arg(long, value_name = "LEFT,TOP,WIDTH,HEIGHT")]
        region: Rect,
    },
}

impl Command {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Navigate { .. } => "navigate",
            Self::Screenshot { .. } => "screenshot",
            Self::Match { .. } => "match",
            Self::MatchImage { .. } => "match-image",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct Rect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

impl FromStr for Rect {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = value.split(',').collect();
        if parts.len() != 4 {
            return Err(format!(
                "expected LEFT,TOP,WIDTH,HEIGHT (4 fields), got {} fields in {value:?}",
                parts.len()
            ));
        }
        let mut numbers = [0; 4];
        for ((number, part), field) in numbers
            .iter_mut()
            .zip(parts)
            .zip(["left", "top", "width", "height"])
        {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!(
                    "rectangle {field}={part:?} must be a decimal unsigned integer"
                ));
            }
            *number = part.parse().map_err(|_| {
                format!(
                    "rectangle {field}={part:?} exceeds the maximum value {}",
                    u32::MAX
                )
            })?;
        }
        if numbers[2] == 0 || numbers[3] == 0 {
            return Err(format!(
                "rectangle width and height must be positive, got width={}, height={} in {value:?}",
                numbers[2], numbers[3]
            ));
        }
        Ok(Self {
            left: numbers[0],
            top: numbers[1],
            width: numbers[2],
            height: numbers[3],
        })
    }
}

impl Rect {
    pub(super) fn validate(self, width: u32, height: u32) -> Result<Region2D<u32>> {
        if u64::from(self.left) + u64::from(self.width) > u64::from(width)
            || u64::from(self.top) + u64::from(self.height) > u64::from(height)
        {
            bail!("rectangle ({self}) must be fully inside the {width}x{height} input image");
        }
        Ok(Region2D::from_ltwh(
            self.left,
            self.top,
            self.width,
            self.height,
        ))
    }
    pub(super) fn from_region(region: Region2D<u32>) -> Self {
        Self {
            left: region.left(),
            top: region.top(),
            width: region.width(),
            height: region.height(),
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "left={}, top={}, width={}, height={}",
            self.left, self.top, self.width, self.height
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum State {
    Overworld,
    Terminal,
    ArchiveMain,
    ArchiveDetail,
    ArchiveMedia,
    ArchiveRecordsPaper,
    ArchiveRecordsDigital,
    ArchiveRecordsCollection,
    ArchiveCentralArchive,
    ArchiveCentralReport,
}

impl State {
    pub(super) fn ui_state(self) -> navigation::UiState {
        match self {
            Self::Overworld => navigation::UiState::Overworld,
            Self::Terminal => navigation::UiState::Terminal,
            Self::ArchiveMain => navigation::UiState::Archive(navigation::ArchiveState::Main),
            Self::ArchiveDetail => navigation::UiState::archive_detail(),
            Self::ArchiveMedia => {
                navigation::UiState::archive_subscene(navigation::ArchiveSubscene::Media)
            }
            Self::ArchiveRecordsPaper => navigation::UiState::archive_subscene(
                navigation::ArchiveSubscene::Records(navigation::RecordsPage::Paper),
            ),
            Self::ArchiveRecordsDigital => navigation::UiState::archive_subscene(
                navigation::ArchiveSubscene::Records(navigation::RecordsPage::Digital),
            ),
            Self::ArchiveRecordsCollection => navigation::UiState::archive_subscene(
                navigation::ArchiveSubscene::Records(navigation::RecordsPage::Collection),
            ),
            Self::ArchiveCentralArchive => navigation::UiState::archive_subscene(
                navigation::ArchiveSubscene::Central(navigation::CentralPage::Archive),
            ),
            Self::ArchiveCentralReport => navigation::UiState::archive_subscene(
                navigation::ArchiveSubscene::Central(navigation::CentralPage::Report),
            ),
        }
    }
}
