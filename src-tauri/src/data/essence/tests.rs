use std::fs;

use serde_json::json;

use crate::app_paths::AppPaths;

use super::Database;

fn fixture() -> anyhow::Result<(tempfile::TempDir, AppPaths)> {
    let root = tempfile::tempdir()?;
    fs::create_dir_all(root.path().join("resources/data"))?;
    fs::write(
        root.path().join("resources/data/essence.json"),
        serde_json::to_vec(&json!({
            "version": 1,
            "terms": [
                {"id": "main", "name": "主能力提升", "kind": "attribute", "sortOrder": 5, "iconId": null},
                {"id": "atk", "name": "攻击提升", "kind": "secondary", "sortOrder": 6, "iconId": null},
                {"id": "tactic", "name": "压制", "kind": "skill", "sortOrder": 19, "iconId": "icon_tactic"}
            ]
        }))?,
    )?;
    fs::write(
        root.path().join("resources/data/weapons.json"),
        serde_json::to_vec(&json!({
            "version": 1,
            "weapons": [
                {"id": "exemplar", "name": "典范", "rarity": 6, "type": "claymore", "iconId": "wpn_claym_0004", "attributeTermId": "main", "secondaryTermId": "atk", "skillTermId": "tactic"},
                {"id": "basic", "name": "基础武器", "rarity": 3, "type": "sword", "iconId": "wpn_sword_0003", "attributeTermId": "main", "secondaryTermId": null, "skillTermId": "tactic"}
            ]
        }))?,
    )?;
    let paths = AppPaths::with_root_dir(root.path());
    Ok((root, paths))
}

#[test]
fn loads_named_weapons_with_optional_secondary_terms() -> anyhow::Result<()> {
    let (_root, paths) = fixture()?;
    let database = Database::load(&paths)?;
    assert_eq!(database.catalog().weapons[0].name, "典范");
    assert_eq!(database.catalog().terms[2].name, "压制");
    assert!(database.catalog().weapons[1].secondary_term_id.is_none());
    Ok(())
}

fn edit_resource(
    paths: &AppPaths,
    file: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) -> anyhow::Result<()> {
    let path = paths.resolve_resource_file(file)?;
    let mut value = serde_json::from_slice(&fs::read(&path)?)?;
    edit(&mut value);
    fs::write(path, serde_json::to_vec(&value)?)?;
    Ok(())
}

#[test]
fn rejects_unknown_term_references() -> anyhow::Result<()> {
    let (_root, paths) = fixture()?;
    edit_resource(&paths, "data/weapons.json", |data| {
        data["weapons"][0]["skillTermId"] = json!("missing");
    })?;
    let error = Database::load(&paths).err().expect("应拒绝不存在的词条");
    assert!(format!("{error:#}").contains("missing"));
    Ok(())
}

#[test]
fn rejects_a_secondary_term_in_the_attribute_slot() -> anyhow::Result<()> {
    let (_root, paths) = fixture()?;
    edit_resource(&paths, "data/weapons.json", |data| {
        data["weapons"][0]["attributeTermId"] = json!("atk");
    })?;
    let error = Database::load(&paths).err().expect("应拒绝词条类别不匹配");
    assert!(format!("{error:#}").contains("类别"));
    Ok(())
}

#[test]
fn rejects_duplicate_catalog_identities() -> anyhow::Result<()> {
    for (file, collection) in [
        ("data/essence.json", "terms"),
        ("data/weapons.json", "weapons"),
    ] {
        let (_root, paths) = fixture()?;
        edit_resource(&paths, file, |data| {
            data[collection][1]["id"] = data[collection][0]["id"].clone();
        })?;
        let error = Database::load(&paths).err().expect("应拒绝重复身份");
        assert!(format!("{error:#}").contains("重复"));
    }
    Ok(())
}

#[test]
fn validates_only_referenced_image_assets() -> anyhow::Result<()> {
    let (root, paths) = fixture()?;
    let directory = root.path().join("resources/assets/itemicon");
    fs::create_dir_all(&directory)?;
    for id in ["icon_tactic", "wpn_claym_0004", "wpn_sword_0003"] {
        image::RgbaImage::new(1, 1).save(directory.join(format!("{id}.png")))?;
    }
    // 未引用的文件不参与检查。
    fs::write(directory.join("unused.png"), b"unused")?;
    let database = Database::load(&paths)?;
    database.validate_assets(&paths)?;
    fs::remove_file(directory.join("icon_tactic.png"))?;
    let error = database
        .validate_assets(&paths)
        .expect_err("应拒绝缺失的被引用图片");
    assert!(format!("{error:#}").contains("icon_tactic"));
    fs::write(directory.join("icon_tactic.png"), b"broken PNG")?;
    let error = database
        .validate_assets(&paths)
        .expect_err("应拒绝损坏的被引用图片");
    assert!(format!("{error:#}").contains("损坏"));
    Ok(())
}

#[test]
fn rejects_unsupported_resource_versions() -> anyhow::Result<()> {
    for file in ["data/essence.json", "data/weapons.json"] {
        let (_root, paths) = fixture()?;
        edit_resource(&paths, file, |data| data["version"] = json!(2))?;
        let error = Database::load(&paths).err().expect("应拒绝不支持的 schema");
        assert!(format!("{error:#}").contains("版本"));
    }
    Ok(())
}

#[test]
fn rejects_missing_display_names() -> anyhow::Result<()> {
    for (file, collection) in [
        ("data/essence.json", "terms"),
        ("data/weapons.json", "weapons"),
    ] {
        let (_root, paths) = fixture()?;
        edit_resource(&paths, file, |data| data[collection][0]["name"] = json!(""))?;
        let error = Database::load(&paths).err().expect("应拒绝缺失的显示名称");
        assert!(format!("{error:#}").contains("名称"));
    }
    Ok(())
}

#[test]
fn real_resources_have_complete_referenced_assets() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let paths = AppPaths::with_root_dir(root);
    let database = Database::load(&paths)?;
    database.validate_assets(&paths)?;
    for weapon in &database.catalog().weapons {
        assert!(database.weapon(&weapon.id).is_some());
        assert!(database.term(&weapon.attribute_term_id).is_some());
    }
    Ok(())
}

#[test]
fn rejects_icon_ids_that_are_not_asset_names() -> anyhow::Result<()> {
    let (_root, paths) = fixture()?;
    edit_resource(&paths, "data/weapons.json", |data| {
        data["weapons"][0]["iconId"] = json!("../outside");
    })?;
    let error = Database::load(&paths).err().expect("应拒绝带目录的图标 ID");
    assert!(format!("{error:#}").contains("图标"));
    Ok(())
}
