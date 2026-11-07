//! 将当前生效设置转换为更新内核的单次操作输入。
use crate::{
    platform,
    settings::{OeaSettings, UpdateProxyMode, UpdateSource},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use tracing::warn;

pub(super) fn from_settings(settings: &OeaSettings) -> oea_update::Options {
    let cdk = configured_cdk(settings);
    let proxy_url = match settings.update_proxy_mode {
        UpdateProxyMode::System => platform::proxy::resolve_system_proxy(),
        UpdateProxyMode::Custom => {
            let url = settings.update_proxy_url.trim();
            Ok((!url.is_empty()).then(|| url.to_owned()))
        }
        UpdateProxyMode::None => Ok(None),
    };
    oea_update::Options {
        source: match settings.update_source {
            UpdateSource::Mirrorchyan => oea_update::UpdateSource::Mirrorchyan,
            UpdateSource::Oem => oea_update::UpdateSource::Oem,
            UpdateSource::Github => oea_update::UpdateSource::Github,
        },
        cdk,
        proxy_url,
    }
}

fn configured_cdk(settings: &OeaSettings) -> Option<String> {
    if settings.update_source != UpdateSource::Mirrorchyan {
        return None;
    }
    let encrypted = settings.mirrorchyan_cdk_encrypted.trim();
    if encrypted.is_empty() {
        return None;
    }

    let blob = match STANDARD.decode(encrypted) {
        Ok(blob) => blob,
        Err(decode_error) => {
            warn!(
                operation = "check",
                error = %decode_error,
                "Mirror酱 CDK 密文 Base64 解码失败，将回退到无 CDK 更新流程"
            );
            return None;
        }
    };
    let plain = match crate::platform::data_protection::decrypt(&blob) {
        Ok(plain) => plain,
        Err(decrypt_error) => {
            warn!(
                operation = "check",
                error = %decrypt_error,
                "Mirror酱 CDK 解密失败，将回退到无 CDK 更新流程"
            );
            return None;
        }
    };
    let plain = match String::from_utf8(plain) {
        Ok(plain) => plain,
        Err(utf8_error) => {
            warn!(
                operation = "check",
                error = %utf8_error,
                "Mirror酱 CDK 明文不是合法 UTF-8，将回退到无 CDK 更新流程"
            );
            return None;
        }
    };
    let plain = plain.trim();
    (!plain.is_empty()).then(|| plain.to_string())
}
