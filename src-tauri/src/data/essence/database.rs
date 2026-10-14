use std::collections::HashMap;

use anyhow::{Context, ensure};

use crate::{app_paths::AppPaths, data::load_resource_json};

use super::{Catalog, Term, TermId, TermKind, Weapon, WeaponId, source};

pub struct Database {
    catalog: Catalog,
    terms: HashMap<TermId, usize>,
    weapons: HashMap<WeaponId, usize>,
}

impl Database {
    pub fn load(app_paths: &AppPaths) -> anyhow::Result<Self> {
        let essence = load_resource_json::<source::Essence>(app_paths, "data/essence.json")?;
        let weapons = load_resource_json::<source::Weapons>(app_paths, "data/weapons.json")?;
        ensure!(
            essence.version == 1,
            "不支持的 essence.json 版本: {}",
            essence.version
        );
        ensure!(
            weapons.version == 1,
            "不支持的 weapons.json 版本: {}",
            weapons.version
        );
        let mut terms = HashMap::new();
        for (i, term) in essence.terms.iter().enumerate() {
            ensure!(!term.name.trim().is_empty(), "词条 {:?} 缺少名称", term.id);
            ensure!(
                terms.insert(term.id.clone(), i).is_none(),
                "词条身份重复: {:?}",
                term.id
            );
        }
        for weapon in &weapons.weapons {
            ensure!(
                !weapon.name.trim().is_empty(),
                "武器 {:?} 缺少名称",
                weapon.id
            );
            for (id, kind) in [
                (&weapon.attribute_term_id, TermKind::Attribute),
                (&weapon.skill_term_id, TermKind::Skill),
            ]
            .into_iter()
            .chain(
                weapon
                    .secondary_term_id
                    .iter()
                    .map(|id| (id, TermKind::Secondary)),
            ) {
                let &index = terms
                    .get(id)
                    .with_context(|| format!("武器 {:?} 引用不存在的词条 {id:?}", weapon.id))?;
                ensure!(
                    essence.terms[index].kind == kind,
                    "武器 {:?} 的词条 {id:?} 类别不匹配，预期 {kind:?}",
                    weapon.id
                );
            }
        }
        for icon in essence
            .terms
            .iter()
            .filter_map(|term| term.icon_id.as_ref())
            .chain(weapons.weapons.iter().map(|weapon| &weapon.icon_id))
        {
            ensure!(
                !icon.0.is_empty()
                    && icon
                        .0
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
                "图标 ID 必须是 itemicon 中的文件名称: {:?}",
                icon.0
            );
        }
        let mut weapon_index = HashMap::new();
        for (i, weapon) in weapons.weapons.iter().enumerate() {
            ensure!(
                weapon_index.insert(weapon.id.clone(), i).is_none(),
                "武器身份重复: {:?}",
                weapon.id
            );
        }
        Ok(Self {
            catalog: Catalog {
                terms: essence
                    .terms
                    .into_iter()
                    .map(|term| Term {
                        id: term.id,
                        name: term.name,
                        kind: term.kind,
                        sort_order: term.sort_order,
                        icon_id: term.icon_id,
                    })
                    .collect(),
                weapons: weapons
                    .weapons
                    .into_iter()
                    .map(|weapon| Weapon {
                        id: weapon.id,
                        name: weapon.name,
                        rarity: weapon.rarity,
                        r#type: weapon.r#type,
                        icon_id: weapon.icon_id,
                        attribute_term_id: weapon.attribute_term_id,
                        secondary_term_id: weapon.secondary_term_id,
                        skill_term_id: weapon.skill_term_id,
                    })
                    .collect(),
            },
            terms,
            weapons: weapon_index,
        })
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn term(&self, id: &TermId) -> Option<&Term> {
        self.terms.get(id).map(|&i| &self.catalog.terms[i])
    }

    pub fn weapon(&self, id: &WeaponId) -> Option<&Weapon> {
        self.weapons.get(id).map(|&i| &self.catalog.weapons[i])
    }

    /// 开发与 CI 检查图片源文件。发布包中的图片由 Vite 提供，不在启动时检查。
    /// 只遍历目录中的外键，不检查未引用的资产。
    pub fn validate_assets(&self, app_paths: &AppPaths) -> anyhow::Result<()> {
        let icons = self
            .catalog
            .terms
            .iter()
            .filter_map(|term| term.icon_id.as_ref())
            .chain(self.catalog.weapons.iter().map(|weapon| &weapon.icon_id));
        for icon in icons {
            let path =
                app_paths.resolve_resource_file(&format!("assets/itemicon/{}.png", icon.0))?;
            image::open(&path).with_context(|| format!("图标图片损坏: {}", path.display()))?;
        }
        Ok(())
    }
}
