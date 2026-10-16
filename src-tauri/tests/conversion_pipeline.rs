mod support;
use ssr2clash_lib::{
    conversion::{ConversionEngine, Converter},
    model::{
        app::{FetchProxy, PresetSource, ProfileConfig},
        clash::ClashConfig,
    },
};
use support::upstream::TestUpstream;

fn profile(origin: &TestUpstream) -> ProfileConfig {
    ProfileConfig {
        source_url: origin.url("/subscribe?token=private-value"),
        ..Default::default()
    }
}

#[tokio::test]
async fn pipeline_returns_structured_counts_and_fetches_each_time() {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/subscribe",
        200,
        include_str!("fixtures/subconverter/input.txt"),
    );
    let engine = ConversionEngine;
    for expected_hits in 1..=2 {
        let result = engine
            .generate(profile(&origin), 73, FetchProxy::Direct)
            .await
            .unwrap();
        assert_eq!(result.output.node_count, 2);
        assert_eq!(result.output.group_count, 10);
        assert_eq!(result.output.invalid_count, 0);
        assert_eq!(result.output.config_revision, 73);
        assert!(
            result.output.generated_at.contains('T') && result.output.generated_at.ends_with('Z')
        );
        let parsed: ClashConfig = serde_saphyr::from_str(&result.output.yaml).unwrap();
        assert_eq!(parsed.proxies.len(), 2);
        assert_eq!(parsed.proxy_groups.len(), 10);
        assert_eq!(result.details.nodes.len(), 2);
        assert_eq!(result.details.yaml, result.output.yaml);
        assert_eq!(origin.hits("/subscribe"), expected_hits);
    }
}

#[tokio::test]
async fn partial_invalid_input_and_filters_produce_truthful_metadata() {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/subscribe",
        200,
        format!(
            "{}\nssr://%%%\n{}",
            support::ssr_link("hk.example.invalid", "香港 01", "测试", true),
            support::ssr_link("us.example.invalid", "美国 01", "测试", false)
        ),
    );
    let mut config = profile(&origin);
    config.exclude_keywords = vec!["美国".into()];
    config.enable_udp = false;
    let result = ConversionEngine
        .generate(config, 1, FetchProxy::Direct)
        .await
        .unwrap();
    assert_eq!(result.output.node_count, 1);
    assert_eq!(result.output.invalid_count, 1);
    assert!(result
        .output
        .warnings
        .iter()
        .all(|warning| !warning.contains("ssr://")));
    assert!(!result.details.nodes[0].udp);
    assert_eq!(result.details.nodes[0].server, "hk.example.invalid");
}

#[tokio::test]
async fn invalid_upstream_or_empty_filter_never_generates_a_configuration() {
    let origin = TestUpstream::new().await;
    let config = profile(&origin);
    origin.set_response("/subscribe", 200, "<html>Sign in</html>");
    assert_eq!(
        ConversionEngine
            .generate(config.clone(), 1, FetchProxy::Direct)
            .await
            .unwrap_err()
            .code,
        "InvalidResponse"
    );
    origin.set_response("/subscribe", 200, "ssr://%%%");
    assert_eq!(
        ConversionEngine
            .generate(config.clone(), 1, FetchProxy::Direct)
            .await
            .unwrap_err()
            .code,
        "NoValidNodes"
    );
    origin.set_response(
        "/subscribe",
        200,
        include_str!("fixtures/subconverter/input.txt"),
    );
    let mut config = config;
    config.filter_keywords = vec!["不存在".into()];
    assert_eq!(
        ConversionEngine
            .generate(config, 1, FetchProxy::Direct)
            .await
            .unwrap_err()
            .code,
        "EmptyFilter"
    );
}

#[tokio::test]
async fn missing_required_rule_fails_the_whole_pipeline() {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/subscribe",
        200,
        include_str!("fixtures/subconverter/input.txt"),
    );
    origin.set_response(
        "/preset.ini",
        200,
        "custom_proxy_group=Proxy`select`[]DIRECT\nruleset=Proxy,missing.list?token=private-rule\n",
    );
    let mut config = profile(&origin);
    config.preset = PresetSource::RemoteIni {
        url: origin.url("/preset.ini"),
    };
    let error = ConversionEngine
        .generate(config, 2, FetchProxy::Direct)
        .await
        .unwrap_err();
    assert_eq!(error.code, "HttpStatus");
    assert!(!error.message.contains("private-rule"));
    assert!(!error.message.contains("private-value"));
    assert_eq!(origin.hits("/subscribe"), 1);
    assert_eq!(origin.hits("/missing.list"), 1);
}

#[tokio::test]
async fn invalid_source_fails_before_any_subscription_work() {
    assert_eq!(
        ConversionEngine
            .generate(ProfileConfig::default(), 0, FetchProxy::Direct)
            .await
            .unwrap_err()
            .code,
        "InvalidSourceUrl"
    );
}
