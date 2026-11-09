use oea_update::{
    Reason,
    source::{AvailableUpdateMetadata, MirrorchyanPackage, Source, resolve_download_plan},
};
use tokio_util::sync::CancellationToken;

fn metadata() -> AvailableUpdateMetadata {
    AvailableUpdateMetadata {
        version_name: "v1.3.0".into(),
        release_note: "notes".into(),
        mirrorchyan_package: Some(MirrorchyanPackage {
            url: "https://example.com/mirror.zip".into(),
            expected_sha256: Some("abc".into()),
            file_size: Some(42),
        }),
    }
}

#[tokio::test]
async fn mirrorchyan_resolves_the_cached_package_without_proxy_configuration() {
    let plan = resolve_download_plan(
        &metadata(),
        Source::Mirrorchyan { cdk: "cdk".into() },
        "test",
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.url, "https://example.com/mirror.zip");
    assert_eq!(plan.filename, "OEA-windows-x86_64-v1.3.0.zip");
    assert_eq!(plan.expected_sha256.as_deref(), Some("abc"));
    assert_eq!(plan.total_size, Some(42));
}

#[tokio::test]
async fn explicit_mirrorchyan_request_without_a_package_does_not_fall_back() {
    let mut metadata = metadata();
    metadata.mirrorchyan_package = None;
    let error = resolve_download_plan(
        &metadata,
        Source::Mirrorchyan { cdk: "cdk".into() },
        "test",
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.reason,
        Reason::PackageUnavailable {
            version: "v1.3.0".into()
        }
    );
}

#[tokio::test]
async fn oem_and_github_reject_an_invalid_proxy_without_cdk_or_source_fallback() {
    for source in [
        Source::Oem {
            proxy_url: Some("http://[invalid".into()),
        },
        Source::Github {
            proxy_url: Some("http://[invalid".into()),
        },
    ] {
        let error = resolve_download_plan(&metadata(), source, "test", &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(error.reason, Reason::ProxyConfiguration);
    }
}
