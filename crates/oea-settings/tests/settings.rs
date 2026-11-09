use oea_settings::{SettingsStore, UiLocale, UpdateSource};
use std::fs;

#[test]
fn migrates_legacy_mirror_without_cdk_and_preserves_other_settings() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.json");
    fs::write(&path, r#"{"majorVersion":0,"minorVersion":1,"updateSource":"mirrorchyan","soundVolume":0.75,"scanTipsDismissedVersion":7}"#).unwrap();
    let store = SettingsStore::open(path.clone(), UiLocale::EnUs).unwrap();
    let settings = store.snapshot();
    assert_eq!(settings.update_source, UpdateSource::Oem);
    assert_eq!(settings.sound_volume, 0.75);
    assert_eq!(settings.scan_tips_dismissed_version, 7);
    assert_eq!(settings.ui_locale, UiLocale::EnUs);
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(saved["minorVersion"], 2);
    assert_eq!(saved["updateSource"], "oem");
}

#[test]
fn migration_preserves_unreadable_ciphertext_and_selected_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.json");
    fs::write(&path, r#"{"majorVersion":0,"minorVersion":1,"updateSource":"mirrorchyan","mirrorchyanCdkEncrypted":"unreadable-ciphertext"}"#).unwrap();
    let settings = SettingsStore::open(path, UiLocale::ZhCn)
        .unwrap()
        .snapshot();
    assert_eq!(settings.update_source, UpdateSource::Mirrorchyan);
    assert_eq!(settings.mirrorchyan_cdk_encrypted, "unreadable-ciphertext");
}

#[test]
fn unversioned_settings_use_historical_defaults_before_migration() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.json");
    fs::write(&path, r#"{"mirrorchyanCdkEncrypted":"saved-cipher"}"#).unwrap();
    let settings = SettingsStore::open(path, UiLocale::EnUs)
        .unwrap()
        .snapshot();
    assert_eq!(settings.update_source, UpdateSource::Mirrorchyan);
    assert_eq!(settings.ui_locale, UiLocale::EnUs);
    assert_eq!(settings.minor_version, 2);
}

#[test]
fn rejects_future_unsupported_and_invalid_files_without_overwriting_them() {
    use oea_settings::SettingsErrorKind;
    for (text, kind) in [
        (
            r#"{"majorVersion":0,"minorVersion":3}"#,
            SettingsErrorKind::NewerVersion,
        ),
        (
            r#"{"majorVersion":0,"minorVersion":0}"#,
            SettingsErrorKind::UnsupportedVersion,
        ),
        (r#"{"majorVersion":0}"#, SettingsErrorKind::InvalidFile),
        ("broken JSON", SettingsErrorKind::InvalidFile),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.json");
        fs::write(&path, text).unwrap();
        let error = SettingsStore::open(path.clone(), UiLocale::ZhCn)
            .err()
            .unwrap();
        assert_eq!(error.kind, kind);
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn persists_mirror_without_cdk_and_preserves_the_source_when_cleared() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.json");
    let store = SettingsStore::open(path.clone(), UiLocale::EnUs).unwrap();
    let mut settings = store.snapshot();
    settings.update_source = UpdateSource::Mirrorchyan;
    store.save(settings.clone()).unwrap();
    let reopened = SettingsStore::open(path.clone(), UiLocale::ZhCn).unwrap();
    assert_eq!(reopened.snapshot().update_source, UpdateSource::Mirrorchyan);
    assert!(reopened.snapshot().mirrorchyan_cdk_encrypted.is_empty());

    settings.mirrorchyan_cdk_encrypted = "cipher".into();
    store.save(settings.clone()).unwrap();
    settings.mirrorchyan_cdk_encrypted.clear();
    store.save(settings).unwrap();
    let restarted = SettingsStore::open(path, UiLocale::ZhCn)
        .unwrap()
        .snapshot();
    assert_eq!(restarted.update_source, UpdateSource::Mirrorchyan);
    assert!(restarted.mirrorchyan_cdk_encrypted.is_empty());
    assert_eq!(restarted.ui_locale, UiLocale::EnUs);
}

#[test]
fn read_failure_can_be_retried_after_repair_and_locale_probe_never_migrates() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.json");
    let original = r#"{"majorVersion":0,"minorVersion":1,"uiLocale":"en-US"}"#;
    fs::write(&path, "invalid").unwrap();
    assert!(SettingsStore::open(path.clone(), UiLocale::ZhCn).is_err());
    fs::write(&path, original).unwrap();
    assert_eq!(
        oea_settings::read_ui_locale(&path).unwrap(),
        Some(UiLocale::EnUs)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(SettingsStore::open(path, UiLocale::ZhCn).is_ok());
}
