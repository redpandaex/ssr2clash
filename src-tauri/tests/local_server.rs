mod support;
use reqwest::{Client, StatusCode};
use ssr2clash_lib::{
    conversion::ConversionEngine, model::app::SavedState, server::ServerController,
    service::AppService,
};
use std::{sync::Arc, time::Duration};
use support::{ssr_link, store::MemoryStore, upstream::TestUpstream};
use tokio::net::TcpListener;

async fn fixture() -> (
    TestUpstream,
    Arc<AppService>,
    Arc<MemoryStore>,
    ServerController,
    Client,
) {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/source",
        200,
        ssr_link("server.example.invalid", "HK test", "test", false),
    );
    let mut saved = SavedState::fresh();
    saved.settings.profile.source_url = origin.url("/source");
    let store = Arc::new(MemoryStore::new(Some(saved)));
    let service = Arc::new(
        AppService::open(store.clone(), Arc::new(ConversionEngine))
            .await
            .unwrap(),
    );
    let controller = ServerController::new(service.clone());
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    (origin, service, store, controller, client)
}

#[tokio::test]
async fn actual_http_gets_refresh_and_head_health_and_invalid_tokens_do_not() {
    let (origin, service, _, server, client) = fixture().await;
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    let base = url.split("/sub/").next().unwrap();
    let health = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    let health = health.text().await.unwrap();
    assert!(!health.contains("source"));
    assert!(!health.contains(&service.subscription_token().await));
    assert_eq!(
        client.head(&url).send().await.unwrap().status(),
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        client
            .get(format!("{base}/sub/wrong.yaml"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(origin.hits("/source"), 0);
    for _ in 0..2 {
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(response.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("yaml"));
        assert!(response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("subscription.yaml"));
        assert!(response
            .headers()
            .get("access-control-allow-origin")
            .is_none());
        assert!(response.text().await.unwrap().contains("proxy-groups:"));
    }
    let response = client.head(&url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.bytes().await.unwrap().is_empty());
    assert_eq!(origin.hits("/source"), 2);
    server.stop().await.unwrap();
}

#[tokio::test]
async fn frozen_http_bytes_survive_reopening_without_contacting_the_origin() {
    let (origin, service, store, server, client) = fixture().await;
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    let original = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    service.set_snapshot(true).await.unwrap();
    origin.set_response("/source", 500, "offline");
    for _ in 0..3 {
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.bytes().await.unwrap(), original);
    }
    server.stop().await.unwrap();
    let reopened = Arc::new(
        AppService::open(store, Arc::new(ConversionEngine))
            .await
            .unwrap(),
    );
    let server = ServerController::new(reopened.clone());
    server.start(0).await.unwrap();
    let url = reopened.status().await.subscription_url.unwrap();
    assert_eq!(
        client.get(url).send().await.unwrap().bytes().await.unwrap(),
        original
    );
    assert_eq!(origin.hits("/source"), 1);
    server.stop().await.unwrap();
}

#[tokio::test]
async fn failure_is_502_until_a_good_result_exists_then_explicitly_stale() {
    let (origin, service, _, server, client) = fixture().await;
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    origin.set_response("/source", 500, "offline");
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        StatusCode::BAD_GATEWAY
    );
    origin.set_response(
        "/source",
        200,
        ssr_link("server.example.invalid", "HK test", "test", false),
    );
    let good = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    origin.set_response("/source", 500, "offline");
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-ssr2clash-stale"], "true");
    assert_eq!(response.bytes().await.unwrap(), good);
    assert!(service.status().await.served_stale);
    server.stop().await.unwrap();
}

#[tokio::test]
async fn two_live_updates_are_allowed_and_stop_cancels_both_and_releases_the_port() {
    let (origin, service, _, server, client) = fixture().await;
    let _gate = origin.block("/source");
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    let port = reqwest::Url::parse(&url).unwrap().port().unwrap();
    let first = tokio::spawn({
        let client = client.clone();
        let url = url.clone();
        async move { client.get(url).send().await }
    });
    let second = tokio::spawn({
        let client = client.clone();
        let url = url.clone();
        async move { client.get(url).send().await }
    });
    origin.wait_for_hits("/source", 2).await;
    let busy = client.get(&url).send().await.unwrap();
    assert_eq!(busy.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(busy.headers()["retry-after"], "1");
    assert_eq!(origin.hits("/source"), 2);
    tokio::time::timeout(Duration::from_secs(4), server.stop())
        .await
        .unwrap()
        .unwrap();
    for response in [
        first.await.unwrap().unwrap(),
        second.await.unwrap().unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    assert!(!service.status().await.server_running);
    assert!(service.wait_idle(Duration::from_secs(1)).await);
    let rebound = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();
    drop(rebound);
}

#[tokio::test]
async fn occupied_port_and_failed_start_save_do_not_claim_a_running_service() {
    let (_, service, store, server, _) = fixture().await;
    let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = occupied.local_addr().unwrap().port();
    assert_eq!(server.start(port).await.unwrap_err().code, "PortInUse");
    assert!(!service.status().await.server_running);
    drop(occupied);
    store.fail_next();
    assert_eq!(server.start(port).await.unwrap_err().code, "StorageWrite");
    assert!(!service.status().await.server_running);
    assert!(!service.should_restore_service().await);
    drop(
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap(),
    );
}

#[tokio::test]
async fn lifecycle_is_idempotent_and_running_port_cannot_change() {
    let (_, service, _, server, _) = fixture().await;
    server.start(0).await.unwrap();
    let original = service.status().await.subscription_url;
    server.start(0).await.unwrap();
    assert_eq!(service.status().await.subscription_url, original);
    let mut settings = service.status().await.settings;
    settings.port += 1;
    assert_eq!(
        service.save_settings(settings).await.unwrap_err().code,
        "ServerMustStop"
    );
    server.stop().await.unwrap();
    server.stop().await.unwrap();
    assert!(!service.should_restore_service().await);
}

#[tokio::test]
async fn failed_stop_save_still_releases_listener_but_reports_the_persistence_error() {
    let (_, service, store, server, _) = fixture().await;
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    let port = reqwest::Url::parse(&url).unwrap().port().unwrap();
    store.fail_next();
    assert_eq!(server.stop().await.unwrap_err().code, "StorageWrite");
    assert!(!service.status().await.server_running);
    assert!(service.should_restore_service().await);
    drop(
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap(),
    );
    server.stop().await.unwrap();
    assert!(!service.should_restore_service().await);
}

#[tokio::test]
async fn missing_source_refuses_to_start_without_leaving_a_listener() {
    let store = Arc::new(MemoryStore::new(Some(SavedState::fresh())));
    let service = Arc::new(
        AppService::open(store, Arc::new(ConversionEngine))
            .await
            .unwrap(),
    );
    let server = ServerController::new(service.clone());
    assert_eq!(server.start(0).await.unwrap_err().code, "InvalidSourceUrl");
    assert!(!service.status().await.server_running);
}

#[tokio::test]
async fn application_shutdown_releases_port_while_preserving_restore_intent() {
    let (_, service, _, server, _) = fixture().await;
    server.start(0).await.unwrap();
    let url = service.status().await.subscription_url.unwrap();
    let port = reqwest::Url::parse(&url).unwrap().port().unwrap();
    server.shutdown().await.unwrap();
    assert!(!service.status().await.server_running);
    assert!(service.should_restore_service().await);
    drop(
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap(),
    );
}
