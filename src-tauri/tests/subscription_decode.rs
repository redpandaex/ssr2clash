mod support;

use base64::{engine::general_purpose, Engine};
use ssr2clash_lib::ssr::parser::parse_ssr_link;
use ssr2clash_lib::subscription::parse_subscription;
use support::ssr_link;

#[test]
fn padded_ssr_fields_preserve_chinese_and_password() {
    let node = parse_ssr_link(&ssr_link("hk.example.invalid", "香港 01", "测试", true)).unwrap();
    assert_eq!(node.server, "hk.example.invalid");
    assert_eq!(node.remarks, "香港 01");
    assert_eq!(node.group, "测试");
    assert_eq!(node.password, "test-password");
}

#[test]
fn ipv6_server_does_not_break_field_splitting() {
    let node = parse_ssr_link(&ssr_link("[2001:db8::1]", "IPv6", "测试", false)).unwrap();
    assert_eq!(node.server, "2001:db8::1");
    assert_eq!(node.server_port, 8388);
}

#[test]
fn zero_port_is_rejected() {
    let body = "example.invalid:0:origin:aes-256-cfb:plain:dGVzdA/?remarks=dGVzdA";
    let link = format!("ssr://{}", general_purpose::URL_SAFE_NO_PAD.encode(body));
    assert!(parse_ssr_link(&link).is_err());
}

#[test]
fn malformed_known_parameter_is_not_silently_erased() {
    let body = "example.invalid:8388:origin:aes-256-cfb:plain:dGVzdA/?remarks=%%%";
    let link = format!("ssr://{}", general_purpose::URL_SAFE_NO_PAD.encode(body));
    assert!(parse_ssr_link(&link).is_err());
}

#[test]
fn plain_and_base64_wrapped_subscriptions_share_one_parser() {
    let plain = format!(
        "{}\nssr://%%%\n{}\n",
        ssr_link("hk.example.invalid", "香港 01", "测试", false),
        ssr_link("us.example.invalid", "美国 01", "测试", true)
    );
    let variants = [
        plain.clone(),
        general_purpose::STANDARD.encode(&plain),
        general_purpose::STANDARD_NO_PAD.encode(&plain),
        general_purpose::URL_SAFE.encode(&plain),
        general_purpose::URL_SAFE_NO_PAD.encode(&plain),
    ];
    for raw in variants {
        let parsed = parse_subscription(&raw).unwrap();
        assert_eq!(parsed.nodes.len(), 2);
        assert_eq!(parsed.invalid_count, 1);
        assert_eq!(parsed.nodes[0].remarks, "香港 01");
        assert_eq!(parsed.nodes[1].remarks, "美国 01");
        assert_eq!(parsed.warnings.len(), 1);
        assert!(!parsed.warnings[0].contains("ssr://"));
    }
}

#[test]
fn html_and_empty_results_are_rejected() {
    for raw in ["<html>Sign in</html>", "", "ssr://%%%"] {
        assert!(parse_subscription(raw).is_err());
    }
}

#[test]
fn percent_encoded_base64_parameters_are_decoded() {
    let body = "example.invalid:8388:origin:aes-256-cfb:plain:dGVzdA%3D%3D/?remarks=5rWL6K%2BV";
    let link = format!("ssr://{}", general_purpose::STANDARD.encode(body));
    let node = parse_ssr_link(&link).unwrap();
    assert_eq!(node.password, "test");
    assert_eq!(node.remarks, "测试");
}
