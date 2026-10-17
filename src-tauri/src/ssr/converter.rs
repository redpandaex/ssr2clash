use crate::model::clash::{ClashConfig, ClashProxy, ClashProxyGroup};
use crate::model::ssr::SsrNode;
use crate::{
    model::app::AppError,
    preset::{
        limits::{check_rule_limits, reserve_checks, GroupBudget, MemberBudget},
        model::ResolvedPreset,
        validate_config, BUILTIN_TARGETS,
    },
};
use fancy_regex::RegexBuilder;
use std::collections::HashSet;

/// 转换 SSR 节点为 Clash 代理
pub fn convert_ssr_to_clash_proxy(ssr: &SsrNode) -> ClashProxy {
    ClashProxy {
        name: ssr.remarks.clone(),
        proxy_type: "ssr".to_string(),
        server: ssr.server.clone(),
        port: ssr.server_port,
        cipher: ssr.method.clone(),
        password: ssr.password.clone(),
        protocol: ssr.protocol.clone(),
        obfs: ssr.obfs.clone(),
        obfs_param: if ssr.obfs_param.is_empty() {
            None
        } else {
            Some(ssr.obfs_param.clone())
        },
        protocol_param: if ssr.protocol_param.is_empty() {
            None
        } else {
            Some(ssr.protocol_param.clone())
        },
        udp: true,
    }
}

pub fn build_clash_config(
    nodes: &[SsrNode],
    preset: &ResolvedPreset,
    enable_udp: bool,
) -> Result<ClashConfig, AppError> {
    let mut definitions = GroupBudget::default();
    for group in &preset.groups {
        definitions.add(group)?;
    }
    check_rule_limits(&preset.rules)?;
    let mut used: HashSet<String> = BUILTIN_TARGETS
        .iter()
        .map(|value| (*value).into())
        .chain(preset.groups.iter().map(|group| group.name.clone()))
        .collect();
    let mut nodes = nodes.to_vec();
    for (index, node) in nodes.iter_mut().enumerate() {
        let base = if node.remarks.trim().is_empty() {
            format!("未命名节点 {}", index + 1)
        } else {
            node.remarks.trim().to_owned()
        };
        let mut name = base.clone();
        let mut suffix = 2;
        while used.contains(&name) {
            name = format!("{base} ({suffix})");
            suffix += 1;
        }
        used.insert(name.clone());
        node.remarks = name;
    }
    let mut groups = Vec::new();
    let mut member_budget = MemberBudget::default();
    let mut match_checks = 0;
    for definition in &preset.groups {
        let mut members = Vec::new();
        let mut seen = HashSet::new();
        for selector in &definition.selectors {
            if let Some(reference) = selector.strip_prefix("[]") {
                if seen.insert(reference) {
                    member_budget.add(reference)?;
                    members.push(reference.to_owned());
                }
                continue;
            }
            let (source_group, pattern) = if let Some(filter) = selector.strip_prefix("!!GROUP=") {
                let (group, pattern) = filter.split_once("!!").unwrap_or((filter, ".*"));
                (Some(group), pattern)
            } else if selector.starts_with("!!") {
                return Err(AppError::new(
                    "UnsupportedPresetOption",
                    "不支持该节点选择条件",
                ));
            } else {
                (None, selector.as_str())
            };
            reserve_checks(
                &mut match_checks,
                nodes
                    .len()
                    .saturating_mul(if source_group.is_some() { 2 } else { 1 }),
            )?;
            let compile = |pattern: &str| {
                RegexBuilder::new(pattern)
                    .backtrack_limit(100_000)
                    .build()
                    .map_err(|_| {
                        AppError::new(
                            "InvalidRegex",
                            format!("策略组「{}」的正则表达式无效", definition.name),
                        )
                    })
            };
            let regex = compile(pattern)?;
            let source_regex = source_group.map(compile).transpose()?;
            for node in &nodes {
                let matches = regex
                    .is_match(&node.remarks)
                    .map_err(|_| AppError::new("InvalidRegex", "节点正则匹配超出执行限制"))?;
                let group_matches = source_regex
                    .as_ref()
                    .map(|regex| regex.is_match(&node.group))
                    .transpose()
                    .map_err(|_| AppError::new("InvalidRegex", "来源组正则匹配失败"))?
                    .unwrap_or(true);
                if matches && group_matches && seen.insert(node.remarks.as_str()) {
                    member_budget.add(&node.remarks)?;
                    members.push(node.remarks.clone());
                }
            }
        }
        groups.push(ClashProxyGroup {
            name: definition.name.clone(),
            group_type: definition.kind.clone(),
            proxies: members,
            url: definition.url.clone(),
            interval: definition.interval,
            timeout: definition.timeout,
            tolerance: definition.tolerance,
        });
    }
    let proxies = nodes
        .iter()
        .map(|node| {
            let mut proxy = convert_ssr_to_clash_proxy(node);
            proxy.udp = enable_udp;
            proxy
        })
        .collect();
    let mut config = ClashConfig {
        port: 7890,
        socks_port: 7891,
        allow_lan: false,
        mode: "rule".to_string(),
        log_level: "info".to_string(),
        proxies,
        proxy_groups: groups,
        rules: preset.rules.clone(),
        extra: serde_json::Map::new(),
    };
    if !preset.base.is_empty() {
        let mut document = serde_json::to_value(&config)
            .map_err(|_| AppError::new("InvalidBaseConfig", "无法合并基础配置"))?;
        for (key, value) in &preset.base {
            if !matches!(key.as_str(), "proxies" | "proxy-groups" | "rules") {
                document
                    .as_object_mut()
                    .unwrap()
                    .insert(key.clone(), value.clone());
            }
        }
        config = serde_json::from_value(document)
            .map_err(|_| AppError::new("InvalidBaseConfig", "基础配置字段类型不兼容"))?;
    }
    validate_config(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_ssr_to_clash_proxy() {
        let ssr = SsrNode {
            server: "example.com".to_string(),
            server_port: 8388,
            protocol: "origin".to_string(),
            method: "aes-256-cfb".to_string(),
            obfs: "plain".to_string(),
            password: "password123".to_string(),
            obfs_param: String::new(),
            protocol_param: String::new(),
            remarks: "测试节点".to_string(),
            group: String::new(),
        };

        let proxy = convert_ssr_to_clash_proxy(&ssr);

        assert_eq!(proxy.name, "测试节点");
        assert_eq!(proxy.proxy_type, "ssr");
        assert_eq!(proxy.server, "example.com");
        assert_eq!(proxy.port, 8388);
    }
}
