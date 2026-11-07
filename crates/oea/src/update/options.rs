//! 在 OEA 中解析设置、选择实际下载源并处理回退。
use crate::{
    platform,
    settings::{OeaSettings, UpdateProxyMode, UpdateSource},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use oea_update::{
    CheckOptions,
    source::{self, AvailableUpdateMetadata},
};
use tracing::warn;

pub(super) fn check_options(settings: &OeaSettings) -> Result<CheckOptions, oea_update::Error> {
    Ok(CheckOptions {
        cdk: configured_cdk(settings),
        proxy_url: resolve_proxy(settings)?,
    })
}

pub(super) fn download_source(
    settings: &OeaSettings,
    metadata: &AvailableUpdateMetadata,
) -> Result<source::Source, oea_update::Error> {
    choose_download_source(
        settings.update_source,
        configured_cdk(settings),
        metadata.mirrorchyan_package.is_some(),
        || resolve_proxy(settings),
    )
}

/// Mirror酱缺少凭证或下载信息时由 OEA 回退到 OEM。选定直连源后不解析代理。
fn choose_download_source(
    configured: UpdateSource,
    cdk: Option<String>,
    has_mirrorchyan_package: bool,
    resolve_proxy: impl FnOnce() -> Result<Option<String>, oea_update::Error>,
) -> Result<source::Source, oea_update::Error> {
    match configured {
        UpdateSource::Mirrorchyan => {
            if let Some(cdk) = cdk
                && has_mirrorchyan_package
            {
                return Ok(source::Source::Mirrorchyan { cdk });
            }
            Ok(source::Source::Oem {
                proxy_url: resolve_proxy()?,
            })
        }
        UpdateSource::Oem => Ok(source::Source::Oem {
            proxy_url: resolve_proxy()?,
        }),
        UpdateSource::Github => Ok(source::Source::Github {
            proxy_url: resolve_proxy()?,
        }),
    }
}

fn resolve_proxy(settings: &OeaSettings) -> Result<Option<String>, oea_update::Error> {
    match settings.update_proxy_mode {
        UpdateProxyMode::System => platform::proxy::resolve_system_proxy().map_err(|error| {
            oea_update::Error::failed(
                oea_update::Reason::ProxyConfiguration,
                anyhow::anyhow!(error),
            )
        }),
        UpdateProxyMode::Custom => {
            let url = settings.update_proxy_url.trim();
            Ok((!url.is_empty()).then(|| url.to_owned()))
        }
        UpdateProxyMode::None => Ok(None),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrorchyan_download_does_not_resolve_a_proxy() {
        let source =
            choose_download_source(UpdateSource::Mirrorchyan, Some("cdk".into()), true, || {
                panic!("Mirror酱下载不应解析代理")
            })
            .unwrap();
        assert!(matches!(source, source::Source::Mirrorchyan { cdk } if cdk == "cdk"));
    }

    #[test]
    fn missing_mirrorchyan_credentials_or_package_selects_oem_with_the_host_proxy() {
        for (cdk, has_package) in [(None, true), (Some("cdk".into()), false)] {
            let source =
                choose_download_source(UpdateSource::Mirrorchyan, cdk, has_package, || {
                    Ok(Some("http://proxy.example:8080".into()))
                })
                .unwrap();
            assert!(
                matches!(source, source::Source::Oem { proxy_url: Some(url) } if url == "http://proxy.example:8080")
            );
        }
    }

    #[test]
    fn explicit_oem_and_github_choices_ignore_mirrorchyan_availability() {
        let oem = choose_download_source(UpdateSource::Oem, None, true, || Ok(None)).unwrap();
        assert!(matches!(oem, source::Source::Oem { proxy_url: None }));
        let github = choose_download_source(UpdateSource::Github, None, true, || {
            Ok(Some("http://proxy.example:8080".into()))
        })
        .unwrap();
        assert!(
            matches!(github, source::Source::Github { proxy_url: Some(url) } if url == "http://proxy.example:8080")
        );
    }

    #[test]
    fn proxy_resolution_failure_stops_the_host_before_it_returns_a_source() {
        for configured in [
            UpdateSource::Mirrorchyan,
            UpdateSource::Oem,
            UpdateSource::Github,
        ] {
            let error = choose_download_source(configured, None, false, || {
                Err(oea_update::Error::new(
                    oea_update::Reason::ProxyConfiguration,
                ))
            })
            .err()
            .unwrap();
            assert_eq!(error.reason, oea_update::Reason::ProxyConfiguration);
        }
    }
}
