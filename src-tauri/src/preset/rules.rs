use crate::model::app::AppError;

pub fn convert_rule(raw: &str, target: &str) -> Result<Option<String>, AppError> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with(['#', ';']) || raw.starts_with("//") {
        return Ok(None);
    }
    let fields: Vec<&str> = raw.split(',').map(str::trim).collect();
    let original = fields[0].to_ascii_uppercase();
    let kind = match original.as_str() {
        "FINAL" => "MATCH",
        "HOST" => "DOMAIN",
        "HOST-SUFFIX" => "DOMAIN-SUFFIX",
        "HOST-KEYWORD" => "DOMAIN-KEYWORD",
        "IP6-CIDR" => "IP-CIDR6",
        value => value,
    };
    if kind == "MATCH" {
        return Ok(Some(format!("MATCH,{target}")));
    }
    if !matches!(
        kind,
        "DOMAIN"
            | "DOMAIN-SUFFIX"
            | "DOMAIN-KEYWORD"
            | "DOMAIN-REGEX"
            | "DOMAIN-WILDCARD"
            | "IP-CIDR"
            | "IP-CIDR6"
            | "SRC-IP-CIDR"
            | "GEOIP"
            | "GEOSITE"
            | "PROCESS-NAME"
            | "PROCESS-PATH"
            | "DST-PORT"
            | "SRC-PORT"
            | "NETWORK"
    ) {
        return Err(AppError::new(
            "UnsupportedRuleType",
            "规则列表中包含当前输出不支持的规则类型",
        ));
    }
    let value = fields
        .get(1)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::new("InvalidRule", "规则缺少匹配内容"))?;
    let no_resolve = fields
        .iter()
        .skip(2)
        .any(|field| field.eq_ignore_ascii_case("no-resolve"));
    Ok(Some(format!(
        "{kind},{value},{target}{}",
        if no_resolve { ",no-resolve" } else { "" }
    )))
}

pub fn parse_rule_list(text: &str, target: &str, format: &str) -> Result<Vec<String>, AppError> {
    let mut raw_lines = Vec::new();
    match format {
        "surge" | "quanx" => raw_lines.extend(
            text.trim_start_matches('\u{feff}')
                .lines()
                .map(str::to_owned),
        ),
        "clash-classic" | "clash-domain" | "clash-ipcidr" => {
            let document: serde_json::Value = serde_saphyr::from_str(text)
                .map_err(|_| AppError::new("InvalidRuleList", "Clash 规则列表不是有效的 YAML"))?;
            let payload = document
                .get("payload")
                .and_then(|value| value.as_array())
                .ok_or_else(|| {
                    AppError::new("InvalidRuleList", "Clash 规则列表缺少 payload 数组")
                })?;
            for item in payload {
                let value = item
                    .as_str()
                    .ok_or_else(|| AppError::new("InvalidRuleList", "规则列表项必须是文本"))?;
                raw_lines.push(match format {
                    "clash-domain" => {
                        if let Some(domain) =
                            value.strip_prefix("+.").or_else(|| value.strip_prefix('.'))
                        {
                            format!("DOMAIN-SUFFIX,{domain}")
                        } else if value.contains(['*', '?']) {
                            format!("DOMAIN-WILDCARD,{value}")
                        } else {
                            format!("DOMAIN,{value}")
                        }
                    }
                    "clash-ipcidr" => format!(
                        "{},{value}",
                        if value.contains(':') {
                            "IP-CIDR6"
                        } else {
                            "IP-CIDR"
                        }
                    ),
                    _ => value.to_owned(),
                });
            }
        }
        _ => {
            return Err(AppError::new(
                "UnsupportedPresetOption",
                "不支持该规则列表格式",
            ))
        }
    }
    let mut rules = Vec::new();
    let mut bytes = 0usize;
    for (index, line) in raw_lines.iter().enumerate() {
        if let Some(rule) =
            convert_rule(line, target).map_err(|error| super::ini::at_line(error, index + 1))?
        {
            bytes = bytes.saturating_add(rule.len());
            if rules.len() >= 100_000 || bytes > 16 * 1024 * 1024 {
                return Err(AppError::new("ImportTooLarge", "展开的规则超过大小限制"));
            }
            rules.push(rule);
        }
    }
    Ok(rules)
}
