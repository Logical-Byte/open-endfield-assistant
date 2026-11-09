//! 将持久化的用户偏好转换为更新内核的一次调用参数。
use crate::{
    platform,
    settings::{OeaSettings, UpdateProxyMode, UpdateSource},
    update::{self, error::Reason},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use oea_update::{CheckOptions, source};

pub(super) fn check_options(settings: &OeaSettings) -> Result<CheckOptions, update::Error> {
    Ok(CheckOptions {
        cdk: configured_cdk(settings)?,
        proxy_url: resolve_proxy(settings)?,
    })
}

pub(super) fn download_source(settings: &OeaSettings) -> Result<source::Source, update::Error> {
    match settings.update_source {
        UpdateSource::Mirrorchyan => Ok(source::Source::Mirrorchyan {
            cdk: configured_cdk(settings)?
                .ok_or_else(|| update::Error::new(Reason::CdkUnavailable))?,
        }),
        UpdateSource::Oem => Ok(source::Source::Oem {
            proxy_url: resolve_proxy(settings)?,
        }),
        UpdateSource::Github => Ok(source::Source::Github {
            proxy_url: resolve_proxy(settings)?,
        }),
    }
}

fn resolve_proxy(settings: &OeaSettings) -> Result<Option<String>, update::Error> {
    match settings.update_proxy_mode {
        UpdateProxyMode::System => platform::proxy::resolve_system_proxy().map_err(|error| {
            update::Error::failed(Reason::ProxyConfiguration, anyhow::anyhow!(error))
        }),
        UpdateProxyMode::Custom => {
            let url = settings.update_proxy_url.trim();
            Ok((!url.is_empty()).then(|| url.to_owned()))
        }
        UpdateProxyMode::None => Ok(None),
    }
}

pub(super) fn configured_cdk(settings: &OeaSettings) -> Result<Option<String>, update::Error> {
    if settings.update_source != UpdateSource::Mirrorchyan {
        return Ok(None);
    }
    let encrypted = settings.mirrorchyan_cdk_encrypted.trim();
    let unavailable =
        |message: &str| update::Error::failed(Reason::CdkUnavailable, anyhow::anyhow!("{message}"));
    if encrypted.is_empty() {
        return Err(unavailable("使用 Mirror酱需要先保存 CDK"));
    }
    let blob = STANDARD
        .decode(encrypted)
        .map_err(|_| unavailable("Mirror酱 CDK 密文无效，请清空后重新填写"))?;
    let plain = crate::platform::data_protection::decrypt(&blob)
        .map_err(|_| unavailable("Mirror酱 CDK 无法解密，请清空后重新填写"))?;
    let plain = String::from_utf8(plain)
        .map_err(|_| unavailable("Mirror酱 CDK 无法读取，请清空后重新填写"))?;
    let plain = plain.trim();
    if plain.is_empty() {
        return Err(unavailable("Mirror酱 CDK 为空，请清空后重新填写"));
    }
    Ok(Some(plain.to_string()))
}
