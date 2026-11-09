use crate::{
    CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION, OeaSettings, SettingsError, SettingsErrorKind,
    UpdateSource,
};

pub(super) fn validate(settings: &OeaSettings) -> Result<(), SettingsError> {
    if (settings.major_version, settings.minor_version)
        != (CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION)
    {
        return Err(SettingsError::new(
            SettingsErrorKind::Validation,
            "只能保存当前版本的设置",
        ));
    }
    if settings.update_source == UpdateSource::Mirrorchyan
        && settings.mirrorchyan_cdk_encrypted.trim().is_empty()
    {
        return Err(SettingsError::new(
            SettingsErrorKind::Validation,
            "使用 Mirror酱需要先保存 CDK",
        ));
    }
    Ok(())
}
