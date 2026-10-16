use crate::{model::ssr::SsrNode, subscription::decode::decode_base64_text};
use anyhow::{anyhow, bail, Context, Result};
use percent_encoding::percent_decode_str;

fn decode_field(value: &str) -> Result<String> {
    let value = percent_decode_str(value)
        .decode_utf8()
        .context("参数不是有效 UTF-8")?;
    decode_base64_text(&value).map_err(Into::into)
}

/// 从右侧拆分固定字段，服务器地址可包含 IPv6 冒号。
pub fn parse_ssr_link(link: &str) -> Result<SsrNode> {
    let encoded = link
        .trim()
        .strip_prefix("ssr://")
        .ok_or_else(|| anyhow!("缺少 SSR 协议前缀"))?;
    let decoded = decode_base64_text(encoded)?;
    let (main, params) = decoded.split_once("/?").unwrap_or((&decoded, ""));
    let parts: Vec<&str> = main.rsplitn(6, ':').collect();
    if parts.len() != 6 {
        bail!("SSR 主体字段不完整");
    }
    let server = parts[5].trim_matches(['[', ']']);
    if server.is_empty() || server.chars().any(char::is_whitespace) || server.contains(['/', '\\'])
    {
        bail!("服务器地址无效");
    }
    if server.contains(':') && server.parse::<std::net::Ipv6Addr>().is_err() {
        bail!("IPv6 地址无效");
    }
    let port: u16 = parts[4].parse().context("端口无效")?;
    if port == 0 {
        bail!("端口不能为 0");
    }
    if parts[1..4].iter().any(|value| value.is_empty()) {
        bail!("协议字段不能为空");
    }
    let mut node = SsrNode {
        server: server.to_owned(),
        server_port: port,
        protocol: parts[3].to_owned(),
        method: parts[2].to_owned(),
        obfs: parts[1].to_owned(),
        password: decode_field(parts[0])?,
        ..SsrNode::default()
    };
    for param in params.split('&') {
        let Some((key, value)) = param.split_once('=') else {
            continue;
        };
        match key {
            "obfsparam" => node.obfs_param = decode_field(value)?,
            "protoparam" => node.protocol_param = decode_field(value)?,
            "remarks" => node.remarks = decode_field(value)?,
            "group" => node.group = decode_field(value)?,
            _ => {}
        }
    }
    Ok(node)
}
