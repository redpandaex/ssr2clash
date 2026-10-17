mod support;
use ssr2clash_lib::{
    conversion::ConversionEngine,
    model::{
        app::{FetchProxy, PresetSource, SavedState},
        clash::ClashConfig,
    },
    preset::{load_builtin_preset, validate_config},
    service::AppService,
    ssr::converter::build_clash_config,
    subscription::parse_subscription,
};
use std::sync::Arc;
use support::{store::MemoryStore, upstream::TestUpstream};

fn config() -> ClashConfig {
    let nodes = parse_subscription(include_str!("fixtures/subconverter/input.txt"))
        .unwrap()
        .nodes;
    build_clash_config(&nodes, &load_builtin_preset().unwrap(), true).unwrap()
}

#[test]
fn final_validation_rejects_unknown_group_types_and_invalid_health_check_values() {
    let mut result = config();
    result.proxy_groups[0].group_type = "unsupported-group".into();
    assert_eq!(
        validate_config(&result).unwrap_err().code,
        "UnsupportedPresetOption"
    );
    for (url, timeout) in [
        ("file:///private", Some(5000)),
        ("https://", Some(5000)),
        ("https://test.example.invalid/204", Some(0)),
    ] {
        let mut result = config();
        let group = &mut result.proxy_groups[1];
        group.url = Some(url.into());
        group.timeout = timeout;
        assert_eq!(
            validate_config(&result).unwrap_err().code,
            "InvalidProxyGroup"
        );
    }
}

#[test]
fn final_validation_rejects_unsupported_or_malformed_routing_rules() {
    for (rule, expected) in [
        ("NOT-A-RULE,anything,DIRECT", "UnsupportedRuleType"),
        ("DOMAIN,DIRECT", "InvalidRule"),
        ("DOMAIN,,DIRECT", "InvalidRule"),
        ("DOMAIN,example.com,DIRECT,unexpected", "InvalidRule"),
        ("MATCH,DIRECT,no-resolve", "InvalidRule"),
        ("IP-CIDR,999.0.0.1/24,DIRECT", "InvalidRule"),
    ] {
        let mut result = config();
        result.rules = vec![rule.into()];
        assert_eq!(
            validate_config(&result).unwrap_err().code,
            expected,
            "{rule}"
        );
    }
    let mut result = config();
    result.rules = vec!["MATCH,DIRECT".into(), "DOMAIN,example.com,DIRECT".into()];
    assert_eq!(
        validate_config(&result).unwrap_err().code,
        "InvalidRuleOrder"
    );
}

#[test]
fn yaml_health_checks_can_be_disabled_with_zero_interval() {
    let mut result = config();
    result.proxy_groups[1].interval = Some(0);
    result.proxy_groups[1].url = None;
    result.proxy_groups[1].timeout = None;
    validate_config(&result).unwrap();
}

#[tokio::test]
async fn yaml_base_unknown_group_options_are_not_silently_discarded() {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/main.ini",
        200,
        "clash_rule_base=base.yaml\nenable_rule_generator=false\n",
    );
    origin.set_response("/base.yaml", 200, "proxy-groups:\n  - name: Proxy\n    type: select\n    proxies: [DIRECT]\n    unknown-option: true\nrules:\n  - MATCH,Proxy\n");
    let error = ssr2clash_lib::preset::resolve::resolve_preset(
        &PresetSource::RemoteIni {
            url: origin.url("/main.ini"),
        },
        &ssr2clash_lib::subscription::http::HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "InvalidBaseConfig");
}

#[tokio::test]
async fn invalid_yaml_base_never_replaces_the_last_good_result() {
    let origin = TestUpstream::new().await;
    origin.set_response(
        "/source",
        200,
        include_str!("fixtures/subconverter/input.txt"),
    );
    origin.set_response(
        "/main.ini",
        200,
        "clash_rule_base=base.yaml\nenable_rule_generator=false\n",
    );
    let base = |kind: &str, rule: &str| {
        format!("proxy-groups:\n  - name: Proxy\n    type: {kind}\n    proxies: [DIRECT]\nrules:\n  - {rule}\n")
    };
    origin.set_response("/base.yaml", 200, base("select", "MATCH,Proxy"));
    let mut saved = SavedState::fresh();
    saved.settings.profile.source_url = origin.url("/source");
    saved.settings.profile.preset = PresetSource::RemoteIni {
        url: origin.url("/main.ini"),
    };
    saved.settings.fetch_proxy = FetchProxy::Direct;
    let store = Arc::new(MemoryStore::new(Some(saved)));
    let service = AppService::open(store.clone(), Arc::new(ConversionEngine))
        .await
        .unwrap();
    let good = service.serve_subscription().await.unwrap();
    for (index, (kind, rule)) in [
        ("unsupported-group", "MATCH,Proxy"),
        ("select", "NOT-A-RULE,anything,DIRECT"),
        ("select", "DOMAIN,DIRECT"),
    ]
    .into_iter()
    .enumerate()
    {
        origin.set_response("/base.yaml", 200, base(kind, rule));
        let fallback = service.serve_subscription().await.unwrap();
        assert!(
            fallback.stale,
            "invalid base {kind}/{rule} must use a marked fallback"
        );
        assert_eq!(fallback.yaml, good.yaml);
        assert_eq!(store.saved().unwrap().last_good.unwrap().yaml, good.yaml);
        assert_eq!(origin.hits("/base.yaml"), index + 2);
    }
}
