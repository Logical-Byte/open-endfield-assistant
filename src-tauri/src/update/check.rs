use base64::{Engine, engine::general_purpose::STANDARD};
use semver::Version;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::config::{OeaConfig, UpdateSource};

use super::http;

const CHECK_URL_BASES: [&str; 2] = [
    "https://mirrorchyan.com/api/resources/OEA/latest",
    "https://mirrorchyan.net/api/resources/OEA/latest",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AvailableUpdateMetadata {
    pub(super) version_name: String,
    pub(super) release_note: String,
    pub(super) mirrorchyan_package: Option<MirrorchyanPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MirrorchyanPackage {
    pub(super) url: String,
    pub(super) expected_sha256: Option<String>,
    pub(super) file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MirrorchyanResponse {
    code: i64,
    msg: String,
    data: Option<MirrorchyanData>,
}

#[derive(Debug, Deserialize)]
struct MirrorchyanData {
    version_name: String,
    release_note: String,
    url: Option<String>,
    sha256: Option<String>,
    filesize: Option<u64>,
}

pub(super) async fn check_for_update(
    config: &OeaConfig,
    current_version: &str,
    user_agent: &str,
) -> Result<Option<AvailableUpdateMetadata>, String> {
    let client = http::build_client(config, user_agent)?;
    let cdk = configured_cdk(config);
    let mut last_response = None;
    let mut last_error = None;

    for base in CHECK_URL_BASES {
        let mut query = vec![
            ("current_version", format!("v{current_version}")),
            ("user_agent", "oea_client".to_string()),
            ("channel", "stable".to_string()),
            ("os", "windows".to_string()),
            ("arch", "amd64".to_string()),
        ];
        if let Some(cdk) = &cdk {
            query.push(("cdk", cdk.clone()));
        }

        let response = match client
            .get(base)
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&query)
            .send()
            .await
        {
            Ok(response) => response,
            Err(request_error) => {
                warn!("{base} 请求失败: {request_error}");
                last_error = Some(request_error.to_string());
                continue;
            }
        };

        let body = match response.bytes().await {
            Ok(body) => body,
            Err(read_error) => {
                warn!("{base} 响应读取失败: {read_error}");
                last_error = Some(read_error.to_string());
                continue;
            }
        };
        let parsed = match serde_json::from_slice::<MirrorchyanResponse>(&body) {
            Ok(parsed) => parsed,
            Err(parse_error) => {
                warn!("{base} 响应解析失败: {parse_error}");
                last_error = Some(parse_error.to_string());
                continue;
            }
        };

        if parsed.code == 0 {
            let metadata = normalize_response(parsed)?;
            if is_newer(&metadata.version_name, current_version) {
                warn!(
                    "检查更新：有新版本可用，当前 v{current_version}，最新 {}",
                    metadata.version_name
                );
                return Ok(Some(metadata));
            }
            info!("检查更新：已是最新版本 v{current_version}");
            return Ok(None);
        }

        warn!("{base} 返回错误 code={}，尝试备用站", parsed.code);
        last_error = Some(format!(
            "Mirror 酱服务返回错误: code={}, msg={}",
            parsed.code, parsed.msg
        ));
        last_response = Some(parsed);
    }

    if let Some(response) = last_response {
        return Err(business_error_message(response.code, &response.msg));
    }

    Err(format!(
        "检查更新请求失败，请检查网络连接或代理设置，或稍后重试。\n{}",
        last_error.unwrap_or_else(|| "未知错误".to_string())
    ))
}

pub(super) fn configured_cdk(config: &OeaConfig) -> Option<String> {
    if config.update_source != UpdateSource::Mirrorchyan {
        return None;
    }
    let encrypted = config.mirrorchyan_cdk_encrypted.trim();
    if encrypted.is_empty() {
        return None;
    }

    let blob = match STANDARD.decode(encrypted) {
        Ok(blob) => blob,
        Err(decode_error) => {
            error!("CDK 密文 Base64 解码失败，更新请求将不携带 CDK: {decode_error}");
            return None;
        }
    };
    let plain = match crate::platform::data_protection::decrypt(&blob) {
        Ok(plain) => plain,
        Err(decrypt_error) => {
            error!("解密 CDK 失败，更新请求将不携带 CDK: {decrypt_error}");
            return None;
        }
    };
    let plain = match String::from_utf8(plain) {
        Ok(plain) => plain,
        Err(utf8_error) => {
            error!("CDK 明文不是合法 UTF-8，更新请求将不携带 CDK: {utf8_error}");
            return None;
        }
    };
    let plain = plain.trim();
    (!plain.is_empty()).then(|| plain.to_string())
}

fn normalize_response(response: MirrorchyanResponse) -> Result<AvailableUpdateMetadata, String> {
    if response.code != 0 {
        return Err(business_error_message(response.code, &response.msg));
    }
    let data = response
        .data
        .ok_or_else(|| "检查更新服务响应异常，请稍后重试".to_string())?;
    let mirrorchyan_package = data.url.and_then(|url| {
        let url = url.trim();
        (!url.is_empty()).then(|| MirrorchyanPackage {
            url: url.to_string(),
            expected_sha256: data.sha256,
            file_size: data.filesize,
        })
    });
    Ok(AvailableUpdateMetadata {
        version_name: data.version_name,
        release_note: data.release_note,
        mirrorchyan_package,
    })
}

fn business_error_message(code: i64, msg: &str) -> String {
    match code {
        1001 => "Mirror酱：请求参数不正确，请联系作者".to_string(),
        7001 => "您的 Mirror酱 CDK 已过期".to_string(),
        7002 => "您的 Mirror酱 CDK 错误，请检查输入是否正确".to_string(),
        7003 => "您的 Mirror酱 CDK 今日下载次数已达上限".to_string(),
        7004 => "您的 Mirror酱 CDK 类型与待下载资源不匹配".to_string(),
        7005 => "您的 Mirror酱 CDK 已被封禁".to_string(),
        8001 => "Mirror酱：对应架构和系统下的资源不存在，请联系作者".to_string(),
        8002 => "Mirror酱：错误的系统参数，请联系作者".to_string(),
        8003 => "Mirror酱：错误的架构参数，请联系作者".to_string(),
        8004 => "Mirror酱：错误的更新通道参数，请联系作者".to_string(),
        code if code < 0 => {
            format!("Mirror 酱服务出现异常，请稍后重试或联系技术支持: {msg}")
        }
        _ if !msg.is_empty() => msg.to_string(),
        _ => format!("未知错误（{code}）"),
    }
}

fn is_newer(latest: &str, current: &str) -> bool {
    let parsed = || -> Result<bool, semver::Error> {
        let latest = Version::parse(latest.trim().strip_prefix('v').unwrap_or(latest.trim()))?;
        let current = Version::parse(current.trim().strip_prefix('v').unwrap_or(current.trim()))?;
        Ok(latest > current)
    };
    parsed().unwrap_or_else(|parse_error| {
        error!("检查更新：版本号比较失败，视为无更新: latest={latest:?}, current={current:?}, error={parse_error}");
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(code: i64, msg: &str, data: Option<MirrorchyanData>) -> MirrorchyanResponse {
        MirrorchyanResponse {
            code,
            msg: msg.to_string(),
            data,
        }
    }

    #[test]
    fn normalizes_only_the_metadata_needed_by_the_backend() {
        let metadata = normalize_response(response(
            0,
            "ok",
            Some(MirrorchyanData {
                version_name: "v1.3.0".to_string(),
                release_note: "notes".to_string(),
                url: Some(" https://example.com/update.zip ".to_string()),
                sha256: Some("abc".to_string()),
                filesize: Some(42),
            }),
        ))
        .unwrap();

        assert_eq!(metadata.version_name, "v1.3.0");
        assert_eq!(metadata.release_note, "notes");
        assert_eq!(
            metadata.mirrorchyan_package,
            Some(MirrorchyanPackage {
                url: "https://example.com/update.zip".to_string(),
                expected_sha256: Some("abc".to_string()),
                file_size: Some(42),
            })
        );
    }

    #[test]
    fn missing_or_empty_package_url_is_not_cached() {
        for url in [None, Some("  ".to_string())] {
            let metadata = normalize_response(response(
                0,
                "ok",
                Some(MirrorchyanData {
                    version_name: "1.3.0".to_string(),
                    release_note: String::new(),
                    url,
                    sha256: Some("ignored".to_string()),
                    filesize: Some(42),
                }),
            ))
            .unwrap();
            assert_eq!(metadata.mirrorchyan_package, None);
        }
    }

    #[test]
    fn response_errors_preserve_existing_user_messages() {
        let known = [
            (1001, "Mirror酱：请求参数不正确，请联系作者"),
            (7001, "您的 Mirror酱 CDK 已过期"),
            (7002, "您的 Mirror酱 CDK 错误，请检查输入是否正确"),
            (7003, "您的 Mirror酱 CDK 今日下载次数已达上限"),
            (7004, "您的 Mirror酱 CDK 类型与待下载资源不匹配"),
            (7005, "您的 Mirror酱 CDK 已被封禁"),
            (8001, "Mirror酱：对应架构和系统下的资源不存在，请联系作者"),
            (8002, "Mirror酱：错误的系统参数，请联系作者"),
            (8003, "Mirror酱：错误的架构参数，请联系作者"),
            (8004, "Mirror酱：错误的更新通道参数，请联系作者"),
        ];
        for (code, expected) in known {
            assert_eq!(business_error_message(code, "service message"), expected);
        }
        assert_eq!(
            business_error_message(-1, "service message"),
            "Mirror 酱服务出现异常，请稍后重试或联系技术支持: service message"
        );
        assert_eq!(
            business_error_message(1, "service message"),
            "service message"
        );
        assert_eq!(business_error_message(2, ""), "未知错误（2）");
        assert_eq!(
            normalize_response(response(0, "ok", None)).unwrap_err(),
            "检查更新服务响应异常，请稍后重试"
        );
    }

    #[test]
    fn compares_semver_and_accepts_the_service_v_prefix() {
        assert!(is_newer("v1.3.0", "1.2.0"));
        assert!(is_newer("1.3.0", "1.2.0"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
        assert!(!is_newer("invalid", "1.2.0"));
        assert!(!is_newer("1.3.0", "invalid"));
    }
}
