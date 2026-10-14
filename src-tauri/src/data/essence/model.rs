use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 游戏中的稳定词条身份。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TS)]
#[ts(export, export_to = "essence/")]
pub struct TermId(String);

/// 游戏中的稳定武器身份。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TS)]
#[ts(export, export_to = "essence/")]
pub struct WeaponId(String);

/// `assets/itemicon/` 中 PNG 文件的名称，不含扩展名。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TS)]
#[ts(export, export_to = "essence/")]
pub struct IconId(pub(super) String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "essence/")]
pub enum TermKind {
    Attribute,
    Secondary,
    Skill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "essence/")]
pub enum WeaponType {
    Sword,
    Claymore,
    Lance,
    Pistol,
    Wand,
}

/// 词条目录条目，数组顺序即显示顺序。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essence/")]
pub struct Term {
    pub id: TermId,
    pub name: String,
    pub kind: TermKind,
    #[ts(type = "number")]
    pub sort_order: i64,
    pub icon_id: Option<IconId>,
}

/// 武器引用三个类别的基质词条，部分武器没有附加属性词条。
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "essence/")]
pub struct Weapon {
    pub id: WeaponId,
    pub name: String,
    pub rarity: u8,
    pub r#type: WeaponType,
    pub icon_id: IconId,
    pub attribute_term_id: TermId,
    pub secondary_term_id: Option<TermId>,
    pub skill_term_id: TermId,
}

/// 前后端共享的只读目录，不包含扫描结果或用户规则。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "essence/")]
pub struct Catalog {
    pub terms: Vec<Term>,
    pub weapons: Vec<Weapon>,
}
