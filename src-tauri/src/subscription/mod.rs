pub mod decode;
pub mod http;
use crate::{
    model::{app::AppError, ssr::SsrNode},
    ssr::parser::parse_ssr_link,
};
use decode::decode_base64_text;

#[derive(Debug)]
pub struct ParsedSubscription {
    pub nodes: Vec<SsrNode>,
    pub invalid_count: usize,
    pub warnings: Vec<String>,
    pub decoded_preview: String,
}

pub fn parse_subscription(raw: &str) -> Result<ParsedSubscription, AppError> {
    let raw = raw.trim().trim_start_matches('\u{feff}').trim();
    if raw.is_empty() || raw.starts_with('<') {
        return Err(AppError::new(
            "InvalidSubscription",
            "订阅内容为空或是网页，未找到 SSR 节点",
        ));
    }
    let decoded = if raw.lines().any(|line| line.trim().starts_with("ssr://")) {
        raw.to_owned()
    } else {
        decode_base64_text(raw)?
    };
    let mut nodes = Vec::new();
    let mut warnings = Vec::new();
    for (index, line) in decoded.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_ssr_link(line) {
            Ok(node) => nodes.push(node),
            Err(_) => warnings.push(format!("第 {} 行不是有效的 SSR 节点，已跳过", index + 1)),
        }
    }
    if nodes.is_empty() {
        return Err(AppError::new("NoValidNodes", "订阅中没有有效的 SSR 节点"));
    }
    Ok(ParsedSubscription {
        nodes,
        invalid_count: warnings.len(),
        warnings,
        decoded_preview: decoded,
    })
}
