use ssr2clash_lib::{
    model::{
        app::{FetchProxy, PresetSource},
        ssr::SsrNode,
    },
    preset::{
        model::{GroupDefinition, ResolvedPreset},
        resolve::resolve_preset,
    },
    ssr::converter::build_clash_config,
    subscription::{http::HttpFetcher, parse_subscription},
};

async fn resolve_text(text: &str) -> Result<ResolvedPreset, ssr2clash_lib::model::app::AppError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.ini");
    std::fs::write(&path, text).unwrap();
    resolve_preset(
        &PresetSource::LocalIni { path },
        &HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
}

#[tokio::test]
async fn repeated_nested_group_imports_fail_before_exponential_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.ini");
    std::fs::write(
        &path,
        "custom_proxy_group=!!import:0.txt\nruleset=DIRECT,[]FINAL\n",
    )
    .unwrap();
    for index in 0..4 {
        std::fs::write(
            dir.path().join(format!("{index}.txt")),
            format!("!!import:{}.txt\n", index + 1).repeat(10),
        )
        .unwrap();
    }
    std::fs::write(dir.path().join("4.txt"), "Proxy`select`[]DIRECT\n").unwrap();
    let error = resolve_preset(
        &PresetSource::LocalIni { path },
        &HttpFetcher::new(FetchProxy::Direct).unwrap(),
    )
    .await
    .err()
    .expect("Repeated imports must fail before building ten thousand duplicate groups");
    assert_eq!(error.code, "DuplicateName");
}

#[tokio::test]
async fn group_count_selector_count_and_definition_bytes_are_bounded() {
    let groups = (0..513)
        .map(|index| format!("custom_proxy_group=G{index}`select`[]DIRECT\n"))
        .collect::<String>();
    let selector_heavy = format!(
        "custom_proxy_group=G`select`{}\n",
        vec!["[]DIRECT"; 8193].join("`")
    );
    let name_heavy = format!(
        "custom_proxy_group={}`select`[]DIRECT\n",
        "x".repeat(1024 * 1024 + 1)
    );
    for definitions in [groups, selector_heavy, name_heavy] {
        let error = resolve_text(&format!("{definitions}ruleset=DIRECT,[]FINAL\n"))
            .await
            .err()
            .expect("Oversized group definitions must fail during resolution");
        assert_eq!(error.code, "ImportTooLarge");
    }
}

fn nodes(count: usize, extra_name_bytes: usize) -> Vec<SsrNode> {
    let source = parse_subscription(include_str!("fixtures/subconverter/input.txt"))
        .unwrap()
        .nodes
        .remove(0);
    (0..count)
        .map(|index| {
            let mut node = source.clone();
            node.remarks = format!("N{index:04}{}", "x".repeat(extra_name_bytes));
            node
        })
        .collect()
}
fn matching_groups(count: usize) -> ResolvedPreset {
    ResolvedPreset {
        groups: (0..count)
            .map(|index| GroupDefinition::parse(&format!("G{index}`select`.*")).unwrap())
            .collect(),
        rules: vec!["MATCH,DIRECT".into()],
        ..ResolvedPreset::default()
    }
}

#[test]
fn matched_member_count_is_bounded_before_serialization() {
    let error = build_clash_config(&nodes(500, 0), &matching_groups(512), true)
        .err()
        .expect("256000 node references must exceed the conversion budget");
    assert_eq!(error.code, "ImportTooLarge");
}

#[test]
fn matched_member_name_bytes_are_bounded_before_serialization() {
    let error = build_clash_config(&nodes(200, 1024), &matching_groups(85), true)
        .err()
        .expect("Repeated long node names must not expand beyond the byte budget");
    assert_eq!(error.code, "ImportTooLarge");
}
