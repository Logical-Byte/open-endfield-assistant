use crate::{
    CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION, OeaSettings, SettingsError, SettingsErrorKind,
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
    Ok(())
}
