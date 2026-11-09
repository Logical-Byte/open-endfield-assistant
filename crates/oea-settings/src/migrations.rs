use crate::{
    CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION, SettingsError, SettingsErrorKind, UiLocale,
};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u32, u32);
const CURRENT: Version = Version(CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION);
struct Migration {
    from: Version,
    to: Version,
    apply: fn(&mut Map<String, Value>, UiLocale) -> Result<(), SettingsError>,
}
const MIGRATIONS: &[Migration] = &[Migration {
    from: Version(0, 1),
    to: Version(0, 2),
    apply: migrate_0_1_to_0_2,
}];

pub(super) fn migrate(document: &mut Value, locale: UiLocale) -> Result<bool, SettingsError> {
    let object = document
        .as_object_mut()
        .ok_or_else(|| invalid("设置文件顶层必须为对象"))?;
    // 无版本的历史文件属于初始 `0.1`，不能使用当前类型的默认版本。
    let mut version = match (object.get("majorVersion"), object.get("minorVersion")) {
        (None, None) => Version(0, 1),
        (Some(major), Some(minor)) => Version(version_number(major)?, version_number(minor)?),
        _ => return Err(invalid("设置文件版本字段不完整")),
    };
    if version > CURRENT {
        return Err(SettingsError::new(
            SettingsErrorKind::NewerVersion,
            format!(
                "设置版本 {}.{} 高于当前支持的 {}.{}，请使用更新版本的应用",
                version.0, version.1, CURRENT.0, CURRENT.1
            ),
        ));
    }
    let original = version;
    while version != CURRENT {
        let migration = MIGRATIONS
            .iter()
            .find(|step| step.from == version)
            .ok_or_else(|| {
                SettingsError::new(
                    SettingsErrorKind::UnsupportedVersion,
                    format!("没有从设置版本 {}.{} 升级的迁移路径", version.0, version.1),
                )
            })?;
        (migration.apply)(object, locale)?;
        version = migration.to;
        object.insert("majorVersion".into(), json!(version.0));
        object.insert("minorVersion".into(), json!(version.1));
    }
    Ok(original != version)
}
fn version_number(value: &Value) -> Result<u32, SettingsError> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| invalid("设置版本必须为非负整数"))
}
fn invalid(message: &str) -> SettingsError {
    SettingsError::new(SettingsErrorKind::InvalidFile, message)
}

fn migrate_0_1_to_0_2(
    object: &mut Map<String, Value>,
    locale: UiLocale,
) -> Result<(), SettingsError> {
    materialize_0_1_defaults(object, locale);
    let cdk = object
        .get("mirrorchyanCdkEncrypted")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("Mirror酱 CDK 密文字段必须为字符串"))?;
    if object.get("updateSource").and_then(Value::as_str) == Some("mirrorchyan")
        && cdk.trim().is_empty()
    {
        object.insert("updateSource".into(), json!("oem"));
    }
    Ok(())
}

/// 已发布的 `0.1` 默认值固定在这里，后续迁移不能引用当前 `OeaSettings::default()`。
fn materialize_0_1_defaults(object: &mut Map<String, Value>, locale: UiLocale) {
    let defaults = json!({
        "uiLocale": locale,
        "minimizeToTray": false,
        "soundVolume": 0.5,
        "updateSource": "mirrorchyan",
        "mirrorchyanCdkEncrypted": "",
        "updateProxyMode": "system",
        "updateProxyUrl": "",
        "autoDownloadUpdates": true,
        "autoInstallUpdates": true,
        "scanTipsDismissedVersion": 0
    });
    for (key, value) in defaults.as_object().expect("静态默认值为对象") {
        object.entry(key.clone()).or_insert_with(|| value.clone());
    }
}
