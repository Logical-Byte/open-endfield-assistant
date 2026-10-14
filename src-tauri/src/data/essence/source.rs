//! 两份 resources 文件的私有读取 schema。

use serde::Deserialize;

use super::{IconId, TermId, TermKind, WeaponId, WeaponType};

#[derive(Deserialize)]
pub(super) struct Essence {
    pub version: u32,
    pub terms: Vec<SourceTerm>,
}

#[derive(Deserialize)]
pub(super) struct Weapons {
    pub version: u32,
    pub weapons: Vec<SourceWeapon>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourceTerm {
    pub id: TermId,
    pub name: String,
    pub kind: TermKind,
    pub sort_order: i64,
    pub icon_id: Option<IconId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SourceWeapon {
    pub id: WeaponId,
    pub name: String,
    pub rarity: u8,
    pub r#type: WeaponType,
    pub icon_id: IconId,
    pub attribute_term_id: TermId,
    pub secondary_term_id: Option<TermId>,
    pub skill_term_id: TermId,
}
