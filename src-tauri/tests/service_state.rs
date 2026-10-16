mod support;
use ssr2clash_lib::{
    model::app::{FetchProxy, OutputMode, SavedState, Theme},
    service::AppService,
};
use std::{sync::Arc, time::Duration};
use support::store::{MemoryStore, TestConverter};

async fn fixture() -> (AppService, Arc<MemoryStore>, Arc<TestConverter>) {
    let mut initial = SavedState::fresh();
    initial.settings.profile.source_url =
        "https://origin.example.invalid/subscribe?token=private-value".into();
    let store = Arc::new(MemoryStore::new(Some(initial)));
    let converter = Arc::new(TestConverter::default());
    let service = AppService::open(store.clone(), converter.clone())
        .await
        .unwrap();
    (service, store, converter)
}

#[tokio::test]
async fn every_live_request_refreshes_while_frozen_requests_reuse_identical_bytes() {
    let (service, _, converter) = fixture().await;
    let first = service.serve_subscription().await.unwrap();
    let second = service.serve_subscription().await.unwrap();
    assert_eq!(converter.calls(), 2);
    assert_ne!(first.yaml, second.yaml);
    service.set_snapshot(true).await.unwrap();
    for _ in 0..3 {
        let result = service.serve_subscription().await.unwrap();
        assert_eq!(result.yaml, second.yaml);
        assert!(!result.stale);
    }
    assert_eq!(converter.calls(), 2);
    assert_eq!(service.refresh().await.unwrap_err().code, "SnapshotFrozen");
    service.set_snapshot(false).await.unwrap();
    service.serve_subscription().await.unwrap();
    assert_eq!(converter.calls(), 3);
}

#[tokio::test]
async fn first_open_creates_a_durable_identity_without_fetching_or_enabling_services() {
    let store = Arc::new(MemoryStore::new(None));
    let converter = Arc::new(TestConverter::default());
    let service = AppService::open(store.clone(), converter.clone())
        .await
        .unwrap();
    let token = store.saved().unwrap().subscription_token;
    assert!(!service.status().await.server_running);
    assert!(!service.should_restore_service().await);
    assert_eq!(
        service.set_snapshot(true).await.unwrap_err().code,
        "NoSnapshotAvailable"
    );
    let reopened = AppService::open(store.clone(), converter.clone())
        .await
        .unwrap();
    assert_eq!(store.saved().unwrap().subscription_token, token);
    assert_eq!(converter.calls(), 0);
    let mut settings = reopened.status().await.settings;
    settings.theme = Theme::Dark;
    reopened.save_settings(settings).await.unwrap();
}

#[tokio::test]
async fn source_failure_uses_only_the_same_revisions_last_good_result() {
    let (service, _, converter) = fixture().await;
    let good = service.serve_subscription().await.unwrap();
    converter.fail_next();
    let fallback = service.serve_subscription().await.unwrap();
    assert!(fallback.stale);
    assert_eq!(fallback.yaml, good.yaml);
    assert!(service.status().await.last_error.is_some());
    let mut settings = service.status().await.settings;
    settings.profile.source_url = "https://origin.example.invalid/new".into();
    service.save_settings(settings).await.unwrap();
    converter.fail_next();
    assert_eq!(
        service.serve_subscription().await.unwrap_err().code,
        "HttpStatus"
    );
    assert!(service.status().await.active_result.is_none());
}

#[tokio::test]
async fn failed_persistence_does_not_publish_new_output_or_change_freeze_mode() {
    let (service, store, _) = fixture().await;
    service.refresh().await.unwrap();
    let previous = service.status().await.active_result;
    let saved = store.saved();
    store.fail_next();
    assert_eq!(service.refresh().await.unwrap_err().code, "StorageWrite");
    assert_eq!(service.status().await.active_result, previous);
    assert_eq!(store.saved(), saved);
    store.fail_next();
    assert_eq!(
        service.set_snapshot(true).await.unwrap_err().code,
        "StorageWrite"
    );
    assert_eq!(service.status().await.mode, OutputMode::Live);
}

#[tokio::test]
async fn freezing_blocks_output_edits_but_allows_theme_changes() {
    let (service, _, _) = fixture().await;
    service.refresh().await.unwrap();
    service.set_snapshot(true).await.unwrap();
    let frozen = service.status().await.active_result;
    let mut settings = service.status().await.settings;
    settings.profile.exclude_keywords = vec!["香港".into()];
    assert_eq!(
        service.save_settings(settings).await.unwrap_err().code,
        "SnapshotFrozen"
    );
    let mut settings = service.status().await.settings;
    settings.theme = Theme::Dark;
    service.save_settings(settings).await.unwrap();
    assert_eq!(service.status().await.mode, OutputMode::Snapshot);
    assert_eq!(service.status().await.active_result, frozen);
}

#[tokio::test]
async fn source_change_cancels_old_generation_and_prevents_late_publication() {
    let (service, _, converter) = fixture().await;
    let gate = converter.block_next();
    let worker = service.clone();
    let job = tokio::spawn(async move { worker.refresh().await });
    gate.wait().await;
    let mut settings = service.status().await.settings;
    settings.profile.source_url = "https://origin.example.invalid/new".into();
    service.save_settings(settings).await.unwrap();
    gate.open();
    assert_eq!(job.await.unwrap().unwrap_err().code, "GenerationCancelled");
    assert!(service.status().await.active_result.is_none());
    service.refresh().await.unwrap();
    assert!(service
        .status()
        .await
        .active_result
        .unwrap()
        .yaml
        .contains("new-"));
}

#[tokio::test]
async fn switching_to_snapshot_keeps_the_frozen_result_when_an_old_request_finishes() {
    let (service, _, converter) = fixture().await;
    let good = service.refresh().await.unwrap();
    let gate = converter.block_next();
    let worker = service.clone();
    let job = tokio::spawn(async move { worker.refresh().await });
    gate.wait().await;
    service.set_snapshot(true).await.unwrap();
    gate.open();
    assert_eq!(job.await.unwrap().unwrap_err().code, "GenerationCancelled");
    assert_eq!(service.serve_subscription().await.unwrap().yaml, good.yaml);
    assert!(!service.status().await.refreshing);
}

#[tokio::test]
async fn stopped_service_intent_and_frozen_link_survive_reopening_without_claiming_to_listen() {
    let (service, store, converter) = fixture().await;
    service.refresh().await.unwrap();
    service.set_snapshot(true).await.unwrap();
    service.set_service_enabled(true).await.unwrap();
    let before = service.status().await;
    let reopened = AppService::open(store.clone(), converter.clone())
        .await
        .unwrap();
    assert!(reopened.should_restore_service().await);
    assert!(!reopened.status().await.server_running);
    assert_eq!(
        reopened.status().await.subscription_url,
        before.subscription_url
    );
    assert_eq!(
        reopened.serve_subscription().await.unwrap().yaml,
        before.active_result.unwrap().yaml
    );
    assert_eq!(converter.calls(), 1);
    reopened.set_service_enabled(false).await.unwrap();
    assert!(
        !AppService::open(store, converter)
            .await
            .unwrap()
            .should_restore_service()
            .await
    );
}

#[tokio::test]
async fn cancelled_caller_does_not_split_disk_commit_from_published_state() {
    let (service, store, _) = fixture().await;
    let gate = store.block_next();
    let worker = service.clone();
    let mut settings = service.status().await.settings;
    settings.port = 26555;
    let mut events = service.subscribe();
    let job = tokio::spawn(async move { worker.save_settings(settings).await });
    gate.wait().await;
    job.abort();
    gate.open();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if service.status().await.settings.port == 26555 {
                break;
            }
            events.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(store.saved().unwrap().settings.port, 26555);
}

#[tokio::test]
async fn newer_same_profile_request_cannot_be_rolled_back_by_an_older_response() {
    let (service, store, converter) = fixture().await;
    let gate = converter.block_next();
    let worker = service.clone();
    let older = tokio::spawn(async move { worker.refresh().await });
    gate.wait().await;
    let newer = service.refresh().await.unwrap();
    gate.open();
    let older_result = older.await.unwrap().unwrap();
    assert_eq!(older_result.yaml, newer.yaml);
    assert_eq!(
        service.status().await.active_result.unwrap().yaml,
        newer.yaml
    );
    assert_eq!(store.saved().unwrap().last_good.unwrap().yaml, newer.yaml);
}

#[tokio::test]
async fn proxy_changes_affect_subsequent_fetches_without_changing_frozen_data() {
    let (service, _, _) = fixture().await;
    let mut settings = service.status().await.settings;
    settings.fetch_proxy = FetchProxy::System;
    service.save_settings(settings).await.unwrap();
    assert!(service.refresh().await.unwrap().yaml.contains("system-"));
}

#[tokio::test]
async fn concurrent_updates_are_limited_and_stop_cancels_them_without_publishing() {
    let (service, _, converter) = fixture().await;
    let first_gate = converter.block_next();
    let worker = service.clone();
    let first = tokio::spawn(async move { worker.refresh().await });
    first_gate.wait().await;
    let second_gate = converter.block_next();
    let worker = service.clone();
    let second = tokio::spawn(async move { worker.refresh().await });
    second_gate.wait().await;
    assert_eq!(service.refresh().await.unwrap_err().code, "TooManyRequests");
    assert_eq!(converter.calls(), 2);
    service.cancel_refreshes().await;
    first_gate.open();
    second_gate.open();
    assert_eq!(
        first.await.unwrap().unwrap_err().code,
        "GenerationCancelled"
    );
    assert_eq!(
        second.await.unwrap().unwrap_err().code,
        "GenerationCancelled"
    );
    assert!(!service.status().await.refreshing);
    assert!(service.status().await.active_result.is_none());
}

#[tokio::test]
async fn runtime_logs_are_bounded_and_never_contain_the_subscription_address() {
    let (service, _, _) = fixture().await;
    for index in 0..120 {
        let mut settings = service.status().await.settings;
        settings.port = 26000 + index;
        service.save_settings(settings).await.unwrap();
    }
    let status = service.status().await;
    assert_eq!(status.logs.len(), 100);
    assert!(status.logs.iter().all(
        |entry| !entry.message.contains("private-value") && !entry.message.contains("https://")
    ));
    assert!(status.logs.windows(2).all(|pair| pair[0].id < pair[1].id));
}

#[tokio::test]
async fn repeated_mode_changes_are_idempotent_and_do_not_fetch() {
    let (service, _, converter) = fixture().await;
    service.set_snapshot(false).await.unwrap();
    service.refresh().await.unwrap();
    service.set_snapshot(true).await.unwrap();
    let frozen = service.status().await.active_result;
    service.set_snapshot(true).await.unwrap();
    assert_eq!(service.status().await.active_result, frozen);
    assert_eq!(converter.calls(), 1);
}

#[tokio::test]
async fn invalid_settings_preserve_the_saved_profile() {
    let (service, store, _) = fixture().await;
    let original = store.saved().unwrap();
    let mut settings = original.settings.clone();
    settings.profile.source_url = format!("http://localhost:{}/sub/token.yaml", settings.port);
    assert_eq!(
        service.save_settings(settings).await.unwrap_err().code,
        "SourceLoop"
    );
    let mut settings = original.settings.clone();
    settings.port = 0;
    assert_eq!(
        service.save_settings(settings).await.unwrap_err().code,
        "InvalidPort"
    );
    assert_eq!(store.saved().unwrap(), original);
}
