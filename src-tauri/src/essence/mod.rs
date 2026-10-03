//! 基质数据与保留规则。识别和游戏操作由自动化模块负责，判断本身只依赖输入数据。

mod evaluation;

use std::sync::OnceLock;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub use evaluation::{Decision, Evaluation, Reason, evaluate};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub stats: Vec<Stat>,
    pub weapons: Vec<Weapon>,
}

impl Catalog {
    pub fn bundled() -> &'static Self {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            serde_json::from_str(include_str!("catalog.json")).expect("内置基质与武器数据应当有效")
        })
    }

    fn stat(&self, id: &str) -> Option<&Stat> {
        self.stats.iter().find(|stat| stat.id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub id: String,
    pub name: String,
    pub kind: StatKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatKind {
    Attribute,
    Secondary,
    Skill,
}

impl StatKind {
    fn index(self) -> usize {
        match self {
            Self::Attribute => 0,
            Self::Secondary => 1,
            Self::Skill => 2,
        }
    }

    fn max_level(self) -> u8 {
        match self {
            Self::Attribute | Self::Secondary => 6,
            Self::Skill => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
    pub id: String,
    pub name: String,
    pub rarity: u8,
    /// 按主属性、次属性、技能排列，与详情面板的视觉顺序无关。
    pub stats: [Option<String>; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Essence {
    /// 保留识别时的行顺序，`levels` 与每行属性一一对应。
    pub stats: [Option<String>; 3],
    pub levels: [Option<u8>; 3],
    pub rarity: Rarity,
    pub locked: Option<bool>,
    pub abandoned: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rarity {
    Five,
    Four,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NonFiveStar {
    #[default]
    Process,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScanSettings {
    pub non_five_star: NonFiveStar,
    pub protect_locked: bool,
    pub skip_abandoned: bool,
    /// 任一类型达标即保留，数组按主属性、次属性、技能排列。
    pub high_level: Option<[u8; 3]>,
    pub excluded_weapon_ids: Vec<String>,
    /// 完整保留组合，数组按主属性、次属性、技能排列。
    pub custom_keeps: Vec<[String; 3]>,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            non_five_star: NonFiveStar::Process,
            protect_locked: true,
            skip_abandoned: false,
            high_level: None,
            excluded_weapon_ids: Vec::new(),
            custom_keeps: Vec::new(),
        }
    }
}

impl ScanSettings {
    pub fn validate(&self, catalog: &Catalog) -> Result<()> {
        let kinds = [StatKind::Attribute, StatKind::Secondary, StatKind::Skill];
        if let Some(thresholds) = self.high_level {
            for (kind, threshold) in kinds.into_iter().zip(thresholds) {
                ensure!(
                    (1..=kind.max_level()).contains(&threshold),
                    "基质高等级保留阈值无效：{threshold}，允许范围为 1..={}",
                    kind.max_level()
                );
            }
        }
        for rule in &self.custom_keeps {
            for (kind, id) in kinds.into_iter().zip(rule) {
                ensure!(
                    catalog.stat(id).is_some_and(|stat| stat.kind == kind),
                    "自定义基质保留组合中的属性不存在或类型不匹配：{id}"
                );
            }
        }
        // 过期武器 ID 不会匹配现有数据，保留它们可避免更新数据后丢失用户设置。
        Ok(())
    }
}
