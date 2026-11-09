use oea_update::{Error, Reason, UpdateOperationStatus, Updater};

#[tokio::test]
async fn failed_host_options_release_the_check_operation() {
    let updater = Updater::default();
    for _ in 0..2 {
        let error = updater
            .check("1.0.0", || {
                assert_eq!(updater.status().operation, UpdateOperationStatus::Checking);
                Err(Error::new(Reason::ProxyConfiguration))
            })
            .await
            .unwrap_err();
        assert_eq!(error.reason, Reason::ProxyConfiguration);
        assert_eq!(updater.status().operation, UpdateOperationStatus::Idle);
    }
}
