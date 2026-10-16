use ssr2clash_lib::{
    model::{clash::ClashConfig, ssr::SsrNode},
    preset::{
        load_builtin_preset,
        model::{GroupDefinition, ResolvedPreset},
        validate_config,
    },
    ssr::converter::{build_clash_config, convert_ssr_to_clash_proxy},
    subscription::parse_subscription,
};

fn sample_nodes() -> Vec<SsrNode> {
    parse_subscription(include_str!("fixtures/subconverter/input.txt"))
        .unwrap()
        .nodes
}
fn default_config() -> ClashConfig {
    build_clash_config(&sample_nodes(), &load_builtin_preset().unwrap(), true).unwrap()
}

#[test]
fn generated_configuration_contains_groups_and_a_routing_fallback() {
    let node = SsrNode {
        server: "hk.example.invalid".into(),
        server_port: 8388,
        remarks: "香港 01".into(),
        method: "aes-256-cfb".into(),
        protocol: "origin".into(),
        obfs: "plain".into(),
        password: "test-password".into(),
        ..Default::default()
    };
    let config = build_clash_config(&[node], &load_builtin_preset().unwrap(), true).unwrap();
    let yaml = serde_saphyr::to_string(&config).unwrap();
    let parsed: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();
    assert!(parsed["proxy-groups"]
        .as_array()
        .is_some_and(|groups| !groups.is_empty()));
    assert_eq!(
        parsed["rules"].as_array().unwrap().last().unwrap(),
        "MATCH,🐟 漏网之鱼"
    );
}

fn normalized_groups(config: &serde_json::Value) -> serde_json::Value {
    let names: std::collections::HashMap<&str, String> = config["proxies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            (
                node["name"].as_str().unwrap(),
                format!("node:{}:{}", node["server"].as_str().unwrap(), node["port"]),
            )
        })
        .collect();
    let mut groups = config["proxy-groups"].clone();
    for group in groups.as_array_mut().unwrap() {
        for candidate in group["proxies"].as_array_mut().unwrap() {
            if let Some(identity) = names.get(candidate.as_str().unwrap()) {
                *candidate = serde_json::Value::String(identity.clone());
            }
        }
    }
    groups
}

#[test]
fn default_groups_and_rule_order_match_official_subconverter_output() {
    let config = default_config();
    let actual = serde_json::to_value(&config).unwrap();
    let expected: serde_json::Value =
        serde_saphyr::from_str(include_str!("fixtures/subconverter/upstream.yaml")).unwrap();
    assert_eq!(config.proxies.len(), 2);
    assert_eq!(config.proxy_groups.len(), 10);
    assert_eq!(normalized_groups(&actual), normalized_groups(&expected));
    let expected_rules = expected["rules"].as_array().unwrap();
    assert_eq!(config.rules.len(), expected_rules.len());
    for (index, (actual, expected)) in config.rules.iter().zip(expected_rules).enumerate() {
        assert_eq!(actual, expected.as_str().unwrap(), "rule {index}");
    }
}

#[test]
fn duplicate_empty_and_reserved_names_cannot_break_references() {
    let mut nodes = vec![sample_nodes()[0].clone(); 4];
    for (node, name) in nodes
        .iter_mut()
        .zip(["🔰 节点选择", "🔰 节点选择", "", "DIRECT"])
    {
        node.remarks = name.into();
    }
    let config = build_clash_config(&nodes, &load_builtin_preset().unwrap(), false).unwrap();
    let names: std::collections::HashSet<_> =
        config.proxies.iter().map(|node| &node.name).collect();
    assert_eq!(names.len(), 4);
    assert!(config.proxies.iter().all(|node| !node.name.is_empty()
        && node.name != "DIRECT"
        && node.name != "🔰 节点选择"
        && !node.udp));
    validate_config(&config).unwrap();
}

#[test]
fn undefined_references_cycles_empty_test_groups_and_duplicate_names_fail() {
    let mut config = default_config();
    config.proxy_groups[0].proxies.push("不存在".into());
    assert_eq!(
        validate_config(&config).unwrap_err().code,
        "InvalidGroupReference"
    );
    let mut config = default_config();
    config.proxy_groups[1].proxies = vec![config.proxy_groups[0].name.clone()];
    assert_eq!(validate_config(&config).unwrap_err().code, "GroupCycle");
    let mut config = default_config();
    config.proxy_groups[1].proxies.clear();
    assert_eq!(
        validate_config(&config).unwrap_err().code,
        "EmptyProxyGroup"
    );
    let mut config = default_config();
    config.proxy_groups.push(config.proxy_groups[0].clone());
    assert_eq!(validate_config(&config).unwrap_err().code, "DuplicateName");
}

#[test]
fn source_group_selectors_and_invalid_regex_are_explicit() {
    let preset = ResolvedPreset {
        groups: vec![GroupDefinition::parse("测试组`select`!!GROUP=测试").unwrap()],
        rules: vec!["MATCH,测试组".into()],
        ..ResolvedPreset::default()
    };
    let config = build_clash_config(&sample_nodes(), &preset, true).unwrap();
    assert_eq!(config.proxy_groups[0].proxies.len(), 2);
    let preset = ResolvedPreset {
        groups: vec![GroupDefinition::parse("测试组`select`[").unwrap()],
        rules: vec!["MATCH,测试组".into()],
        ..ResolvedPreset::default()
    };
    assert_eq!(
        build_clash_config(&sample_nodes(), &preset, true)
            .unwrap_err()
            .code,
        "InvalidRegex"
    );
}

#[test]
fn ssr_parameters_use_client_compatible_yaml_keys() {
    let node = SsrNode {
        server: "hk.example.invalid".into(),
        server_port: 8388,
        obfs_param: "host.example.invalid".into(),
        protocol_param: "123:sample".into(),
        ..Default::default()
    };
    let yaml = serde_saphyr::to_string(&convert_ssr_to_clash_proxy(&node)).unwrap();
    assert!(yaml.contains("obfs-param:"));
    assert!(yaml.contains("protocol-param:"));
    assert!(!yaml.contains("obfs_param:"));
}
