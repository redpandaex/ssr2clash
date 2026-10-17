use crate::model::app::AppError;

pub(crate) fn supported_rule_type(kind: &str) -> bool {
    matches!(
        kind,
        "MATCH"
            | "DOMAIN"
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
    )
}

pub(crate) fn validate_rule(rule: &str) -> Result<&str, AppError> {
    let fields: Vec<_> = rule.split(',').map(str::trim).collect();
    let kind = fields[0];
    if !supported_rule_type(kind) {
        return Err(AppError::new(
            "UnsupportedRuleType",
            "配置包含当前输出不支持的规则类型",
        ));
    }
    let invalid = || AppError::new("InvalidRule", "分流规则的匹配内容、策略或附加参数无效");
    if kind == "MATCH" {
        return if fields.len() == 2 && !fields[1].is_empty() {
            Ok(fields[1])
        } else {
            Err(invalid())
        };
    }
    if !(3..=4).contains(&fields.len()) || fields[1].is_empty() || fields[2].is_empty() {
        return Err(invalid());
    }
    if fields.len() == 4
        && (fields[3] != "no-resolve" || !matches!(kind, "IP-CIDR" | "IP-CIDR6" | "GEOIP"))
    {
        return Err(invalid());
    }
    if matches!(kind, "IP-CIDR" | "IP-CIDR6" | "SRC-IP-CIDR") {
        let (address, prefix) = fields[1].split_once('/').ok_or_else(invalid)?;
        let address: std::net::IpAddr = address.parse().map_err(|_| invalid())?;
        let prefix: u8 = prefix.parse().map_err(|_| invalid())?;
        if prefix > if address.is_ipv4() { 32 } else { 128 } {
            return Err(invalid());
        }
    }
    if matches!(kind, "DST-PORT" | "SRC-PORT") {
        for range in fields[1].split('/') {
            let (start, end) = range.split_once('-').unwrap_or((range, range));
            let start: u16 = start.parse().map_err(|_| invalid())?;
            let end: u16 = end.parse().map_err(|_| invalid())?;
            if start > end {
                return Err(invalid());
            }
        }
    }
    if kind == "NETWORK"
        && !fields[1].eq_ignore_ascii_case("tcp")
        && !fields[1].eq_ignore_ascii_case("udp")
    {
        return Err(invalid());
    }
    Ok(fields[2])
}

fn separate_final(rules: Vec<String>) -> Result<(Vec<String>, Option<String>), AppError> {
    let mut ordinary = Vec::new();
    let mut final_rule = None;
    for rule in rules {
        let fields: Vec<_> = rule.split(',').map(str::trim).collect();
        if fields.first() == Some(&"MATCH") {
            if fields.len() != 2 || fields[1].is_empty() {
                return Err(AppError::new(
                    "InvalidRule",
                    "MATCH 规则必须只包含一个有效策略",
                ));
            }
            let canonical = format!("MATCH,{}", fields[1]);
            if final_rule
                .as_ref()
                .is_some_and(|existing| existing != &canonical)
            {
                return Err(AppError::new(
                    "ConflictingFinalRule",
                    "配置包含不同的 MATCH 兜底策略",
                ));
            }
            final_rule = Some(canonical);
        } else {
            if final_rule.is_some() {
                return Err(AppError::new(
                    "InvalidRuleOrder",
                    "MATCH 兜底规则之后不能再有匹配规则",
                ));
            }
            ordinary.push(rule);
        }
    }
    Ok((ordinary, final_rule))
}

pub fn merge_rules(base: Vec<String>, generated: Vec<String>) -> Result<Vec<String>, AppError> {
    let (mut base, base_final) = separate_final(base)?;
    let (generated, generated_final) = separate_final(generated)?;
    if let (Some(base), Some(generated)) = (&base_final, &generated_final) {
        if base != generated {
            return Err(AppError::new("ConflictingFinalRule", "基础配置与生成规则的 MATCH 策略冲突，请设置 overwrite_original_rules=true 或使用相同兜底策略"));
        }
    }
    base.extend(generated);
    if let Some(final_rule) = base_final.or(generated_final) {
        base.push(final_rule);
    }
    Ok(base)
}

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
    if !supported_rule_type(kind) {
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
