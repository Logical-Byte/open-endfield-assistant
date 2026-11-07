use super::*;
use crate::storage::CachedJsonFile;
use serde::{Deserialize, Serialize};

pub(super) const ACCENTS: [(&str, u32); 5] = [
    ("Pink", 0xec4899),
    ("Blue", 0x3b82f6),
    ("Green", 0x22c55e),
    ("Orange", 0xf97316),
    ("Purple", 0xa855f7),
];

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Appearance {
    pub dark: Option<bool>,
    pub collapsed: bool,
    pub accent: usize,
}

pub(super) fn load(paths: &AppPaths) -> Appearance {
    CachedJsonFile::<Appearance>::at(paths.config_dir().join("appearance.json"))
        .map(|file| file.snapshot().as_ref().clone())
        .unwrap_or_default()
}
pub(super) fn save(appearance: &Appearance) -> anyhow::Result<()> {
    let paths = AppPaths::new().map_err(anyhow::Error::msg)?;
    CachedJsonFile::new(
        paths.config_dir().join("appearance.json"),
        appearance.clone(),
    )
    .replace(appearance.clone())
}
pub(super) fn apply(appearance: &Appearance, window: Option<&mut Window>, cx: &mut App) {
    if let Some(dark) = appearance.dark {
        Theme::change(
            if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            window,
            cx,
        );
    }
    ui::configure_theme(cx);
    let color = ACCENTS[appearance.accent % ACCENTS.len()].1;
    Theme::global_mut(cx).primary = rgb(color).into();
}
