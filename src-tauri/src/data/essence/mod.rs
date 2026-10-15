//! 基质词条与武器目录。原表关联由生成脚本完成。

mod database;
mod model;
mod source;

pub use database::Database;
pub use model::{Catalog, IconId, Term, TermId, TermKind, Weapon, WeaponId, WeaponType};

#[cfg(test)]
mod tests;
