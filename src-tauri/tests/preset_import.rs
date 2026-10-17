mod support;
use ssr2clash_lib::{
    model::app::{FetchProxy, PresetSource},
    preset::resolve::resolve_preset,
    subscription::http::HttpFetcher,
};
use support::upstream::TestUpstream;

#[tokio::test]
async fn remote_ini_preserves_repeated_groups_rules_and_relative_urls() {
    let origin = TestUpstream::new().await;
    origin.set_response("/config/main.ini", 200, "[custom]\ncustom_proxy_group=Proxy`select`[]DIRECT`.*\ncustom_proxy_group=AUTO`url-test`.*`https://example.invalid/204`300,5,50\nruleset=Proxy,../rules.list\nruleset=Proxy,[]FINAL\n");
    origin.set_response(
        "/rules.list",
        200,
        "# test\nDOMAIN-SUFFIX,example.com\nIP-CIDR,10.0.0.0/8,no-resolve\n",
    );
    let preset = resolve_preset(
        &PresetSource::RemoteIni {
            url: origin.url("/config/main.ini"),
        },
        &HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(preset.groups.len(), 2);
    assert_eq!(preset.groups[0].name, "Proxy");
    assert_eq!(preset.groups[1].timeout, Some(5000));
    assert_eq!(preset.groups[1].tolerance, Some(50));
    assert_eq!(
        preset.rules,
        [
            "DOMAIN-SUFFIX,example.com,Proxy",
            "IP-CIDR,10.0.0.0/8,Proxy,no-resolve",
            "MATCH,Proxy"
        ]
    );
    assert_eq!(origin.hits("/rules.list"), 1);
}

#[tokio::test]
async fn local_imports_expand_within_the_selected_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("parts")).unwrap();
    std::fs::write(dir.path().join("main.ini"), "[custom]\ncustom_proxy_group=!!import:parts/groups.txt\nruleset=!!import:parts/rulesets.txt\n").unwrap();
    std::fs::write(
        dir.path().join("parts/groups.txt"),
        "Proxy`select`[]DIRECT\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("parts/rulesets.txt"),
        "Proxy,domains.list\nProxy,[]FINAL\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("parts/domains.list"),
        "DOMAIN,example.com\n",
    )
    .unwrap();
    let preset = resolve_preset(
        &PresetSource::LocalIni {
            path: dir.path().join("main.ini"),
        },
        &HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(preset.groups[0].name, "Proxy");
    assert_eq!(preset.rules, ["DOMAIN,example.com,Proxy", "MATCH,Proxy"]);
}

#[tokio::test]
async fn cyclic_import_and_local_path_escape_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("allowed")).unwrap();
    let main = dir.path().join("allowed/main.ini");
    let source = PresetSource::LocalIni { path: main.clone() };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    std::fs::write(&main, "custom_proxy_group=!!import:groups.txt\n").unwrap();
    std::fs::write(
        dir.path().join("allowed/groups.txt"),
        "!!import:groups.txt\n",
    )
    .unwrap();
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap_err().code,
        "ImportCycle"
    );
    std::fs::write(
        &main,
        "custom_proxy_group=Proxy`select`[]DIRECT\nruleset=Proxy,../outside.list\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("outside.list"), "DOMAIN,private.invalid\n").unwrap();
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap_err().code,
        "ResourceOutsideRoot"
    );
}

#[tokio::test]
async fn remote_config_cannot_read_local_files_and_unknown_effective_options_fail() {
    let origin = TestUpstream::new().await;
    let source = PresetSource::RemoteIni {
        url: origin.url("/config.ini"),
    };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    origin.set_response(
        "/config.ini",
        200,
        "custom_proxy_group=Proxy`select`[]DIRECT\nruleset=Proxy,file:///C:/Windows/win.ini\n",
    );
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap_err().code,
        "UnsupportedResource"
    );
    origin.set_response("/config.ini", 200, "[custom]\nrename=secret@value\n");
    let error = resolve_preset(&source, &fetcher).await.unwrap_err();
    assert_eq!(error.code, "UnsupportedPresetOption");
    assert!(error.message.contains('2'));
}

#[tokio::test]
async fn failed_required_rule_is_an_error_and_credentials_are_not_reported() {
    let origin = TestUpstream::new().await;
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    origin.set_response(
        "/config.ini",
        200,
        "custom_proxy_group=Proxy`select`[]DIRECT\nruleset=Proxy,rules.list?token=secret-value\n",
    );
    origin.set_response("/rules.list", 503, "upstream error");
    let error = resolve_preset(
        &PresetSource::RemoteIni {
            url: origin.url("/config.ini"),
        },
        &fetcher,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "HttpStatus");
    assert!(!error.message.contains("secret-value"));
    assert_eq!(origin.hits("/rules.list"), 1);
}

#[tokio::test]
async fn oversized_or_html_response_is_rejected_without_retry() {
    let origin = TestUpstream::new().await;
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    origin.set_response("/large", 200, "x".repeat(8 * 1024 * 1024 + 1));
    assert_eq!(
        fetcher
            .get_text(&origin.url("/large"))
            .await
            .unwrap_err()
            .code,
        "ResponseTooLarge"
    );
    origin.set_response("/html", 200, "<html>Sign in</html>");
    assert_eq!(
        fetcher
            .get_text(&origin.url("/html"))
            .await
            .unwrap_err()
            .code,
        "InvalidResponse"
    );
    assert_eq!(origin.hits("/large"), 1);
}

#[tokio::test]
async fn timeout_is_bounded_and_does_not_retry() {
    let origin = TestUpstream::new().await;
    origin.set_response("/slow", 200, "result");
    let _gate = origin.block("/slow");
    let url = origin.url("/slow?token=secret-value");
    let fetch = tokio::spawn(async move {
        HttpFetcher::new(FetchProxy::Direct)
            .unwrap()
            .get_text(&url)
            .await
    });
    origin.wait_for_hits("/slow", 1).await;
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(21)).await;
    let error = fetch.await.unwrap().unwrap_err();
    assert_eq!(error.code, "UpstreamTimeout");
    assert!(!error.message.contains("secret-value"));
    assert_eq!(origin.hits("/slow"), 1);
}

#[tokio::test]
async fn plain_yaml_base_obeys_generator_and_overwrite_switches() {
    let dir = tempfile::tempdir().unwrap();
    let origin = TestUpstream::new().await;
    origin.set_response("/rules", 200, "DOMAIN-SUFFIX,remote.invalid\n");
    std::fs::write(
        dir.path().join("base.yaml"),
        "dns:\n  enable: false\nrules:\n  - DOMAIN,local.invalid,DIRECT\n  - MATCH,Proxy\n",
    )
    .unwrap();
    let ini = |enabled, overwrite| {
        format!("custom_proxy_group=Proxy`select`[]DIRECT\nclash_rule_base=base.yaml\nenable_rule_generator={enabled}\noverwrite_original_rules={overwrite}\nruleset=Proxy,{}\nruleset=Proxy,[]FINAL\n", origin.url("/rules"))
    };
    let path = dir.path().join("main.ini");
    let source = PresetSource::LocalIni { path: path.clone() };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    std::fs::write(&path, ini(false, false)).unwrap();
    let preset = resolve_preset(&source, &fetcher).await.unwrap();
    assert_eq!(preset.rules, ["DOMAIN,local.invalid,DIRECT", "MATCH,Proxy"]);
    assert_eq!(origin.hits("/rules"), 0);
    let nodes = ssr2clash_lib::subscription::parse_subscription(include_str!(
        "fixtures/subconverter/input.txt"
    ))
    .unwrap()
    .nodes;
    let config = ssr2clash_lib::ssr::converter::build_clash_config(&nodes, &preset, true).unwrap();
    assert_eq!(
        serde_json::to_value(config).unwrap()["dns"]["enable"],
        false
    );
    std::fs::write(&path, ini(true, false)).unwrap();
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap().rules,
        [
            "DOMAIN,local.invalid,DIRECT",
            "DOMAIN-SUFFIX,remote.invalid,Proxy",
            "MATCH,Proxy"
        ]
    );
    std::fs::write(&path, ini(true, true)).unwrap();
    let preset = resolve_preset(&source, &fetcher).await.unwrap();
    assert_eq!(
        preset.rules,
        ["DOMAIN-SUFFIX,remote.invalid,Proxy", "MATCH,Proxy"]
    );
}

#[tokio::test]
async fn appended_generated_rules_stay_reachable_before_one_final_rule() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("base.yaml"),
        "rules:\n  - DOMAIN,local.invalid,DIRECT\n  - MATCH,DIRECT\n",
    )
    .unwrap();
    let path = dir.path().join("main.ini");
    std::fs::write(&path, "custom_proxy_group=Proxy`select`[]DIRECT\nclash_rule_base=base.yaml\nruleset=REJECT,[]DOMAIN-SUFFIX,example.com\nruleset=DIRECT,[]FINAL\n").unwrap();
    let preset = resolve_preset(
        &PresetSource::LocalIni { path },
        &HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        preset.rules,
        [
            "DOMAIN,local.invalid,DIRECT",
            "DOMAIN-SUFFIX,example.com,REJECT",
            "MATCH,DIRECT"
        ]
    );
}

#[tokio::test]
async fn conflicting_base_and_generated_fallbacks_fail_explicitly_unless_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("base.yaml"), "rules:\n  - MATCH,DIRECT\n").unwrap();
    let path = dir.path().join("main.ini");
    let ini = "custom_proxy_group=Proxy`select`[]DIRECT\nclash_rule_base=base.yaml\nruleset=Proxy,[]FINAL\n";
    let source = PresetSource::LocalIni { path: path.clone() };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    std::fs::write(&path, ini).unwrap();
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap_err().code,
        "ConflictingFinalRule"
    );
    std::fs::write(&path, format!("{ini}overwrite_original_rules=true\n")).unwrap();
    assert_eq!(
        resolve_preset(&source, &fetcher).await.unwrap().rules,
        ["MATCH,Proxy"]
    );
}

#[tokio::test]
async fn shared_remote_rule_is_fetched_once_per_conversion_but_not_cached_across_conversions() {
    let origin = TestUpstream::new().await;
    origin.set_response("/main.ini", 200, "custom_proxy_group=A`select`[]DIRECT\ncustom_proxy_group=B`select`[]DIRECT\nruleset=A,shared.list\nruleset=B,shared.list\nruleset=A,[]FINAL\n");
    origin.set_response("/shared.list", 200, "DOMAIN-SUFFIX,example.com\n");
    let source = PresetSource::RemoteIni {
        url: origin.url("/main.ini"),
    };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    for expected in 1..=2 {
        let preset = resolve_preset(&source, &fetcher).await.unwrap();
        assert_eq!(preset.rules[0], "DOMAIN-SUFFIX,example.com,A");
        assert_eq!(preset.rules[1], "DOMAIN-SUFFIX,example.com,B");
        assert_eq!(origin.hits("/main.ini"), expected);
        assert_eq!(origin.hits("/shared.list"), expected);
    }
}

#[tokio::test]
async fn import_depth_and_total_resource_budget_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main.ini");
    std::fs::write(
        &main,
        "custom_proxy_group=!!import:0.txt\nruleset=Proxy,[]FINAL\n",
    )
    .unwrap();
    for index in 0..10 {
        std::fs::write(
            dir.path().join(format!("{index}.txt")),
            format!("!!import:{}.txt\n", index + 1),
        )
        .unwrap();
    }
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    assert_eq!(
        resolve_preset(&PresetSource::LocalIni { path: main.clone() }, &fetcher)
            .await
            .unwrap_err()
            .code,
        "ImportDepth"
    );
    std::fs::write(&main, "custom_proxy_group=!!import:large0.txt\ncustom_proxy_group=!!import:large1.txt\ncustom_proxy_group=!!import:large2.txt\nruleset=Proxy,[]FINAL\n").unwrap();
    let padding = format!("#{}\nProxy`select`[]DIRECT\n", "x".repeat(6 * 1024 * 1024));
    for index in 0..3 {
        std::fs::write(
            dir.path().join(format!("large{index}.txt")),
            padding.replace("Proxy`", &format!("Proxy{index}`")),
        )
        .unwrap();
    }
    assert_eq!(
        resolve_preset(&PresetSource::LocalIni { path: main }, &fetcher)
            .await
            .unwrap_err()
            .code,
        "ImportTooLarge"
    );
}

#[tokio::test]
async fn typed_rule_lists_and_invalid_regex_have_explicit_semantics() {
    let origin = TestUpstream::new().await;
    origin.set_response("/main.ini", 200, "custom_proxy_group=Proxy`select`[]DIRECT\nruleset=Proxy,clash-domain:domains.yml\nruleset=Proxy,clash-ipcidr:ips.yml\nruleset=Proxy,quanx:quanx.list\nruleset=Proxy,[]FINAL\n");
    origin.set_response(
        "/domains.yml",
        200,
        "payload:\n  - +.example.com\n  - exact.invalid\n",
    );
    origin.set_response(
        "/ips.yml",
        200,
        "payload:\n  - 192.0.2.0/24\n  - 2001:db8::/32\n",
    );
    origin.set_response("/quanx.list", 200, "HOST-SUFFIX,quanx.invalid,OldPolicy\n");
    let source = PresetSource::RemoteIni {
        url: origin.url("/main.ini"),
    };
    let fetcher = HttpFetcher::new(FetchProxy::Direct).unwrap();
    let preset = resolve_preset(&source, &fetcher).await.unwrap();
    assert_eq!(
        preset.rules,
        [
            "DOMAIN-SUFFIX,example.com,Proxy",
            "DOMAIN,exact.invalid,Proxy",
            "IP-CIDR,192.0.2.0/24,Proxy",
            "IP-CIDR6,2001:db8::/32,Proxy",
            "DOMAIN-SUFFIX,quanx.invalid,Proxy",
            "MATCH,Proxy"
        ]
    );
    origin.set_response(
        "/main.ini",
        200,
        "[custom]\ncustom_proxy_group=Broken`select`[\nruleset=Broken,[]FINAL\n",
    );
    let error = resolve_preset(&source, &fetcher).await.unwrap_err();
    assert_eq!(error.code, "InvalidRegex");
    assert!(error.message.contains('2'));
}
