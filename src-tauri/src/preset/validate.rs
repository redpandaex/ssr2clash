use super::BUILTIN_TARGETS;
use crate::model::{app::AppError, clash::ClashConfig};
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) fn validate_group_options(
    kind: &str,
    url: Option<&str>,
    interval: Option<u64>,
    timeout: Option<u64>,
) -> Result<(), AppError> {
    if !matches!(kind, "select" | "url-test" | "fallback" | "load-balance") {
        return Err(AppError::new(
            "UnsupportedPresetOption",
            "不支持该策略组类型",
        ));
    }
    if url.is_some_and(|url| crate::subscription::http::parse_http_url(url).is_err())
        || timeout == Some(0)
        || (kind != "select" && interval.is_some_and(|value| value > 0) && url.is_none())
    {
        return Err(AppError::new(
            "InvalidProxyGroup",
            "健康检查需要有效 HTTP(S) 地址和正数超时",
        ));
    }
    Ok(())
}

pub fn validate_config(config: &ClashConfig) -> Result<(), AppError> {
    super::limits::check_group_count(config.proxy_groups.len())?;
    super::limits::check_rule_limits(&config.rules)?;
    let mut members = super::limits::MemberBudget::default();
    for group in &config.proxy_groups {
        for member in &group.proxies {
            members.add(member)?;
        }
    }
    if config.proxies.is_empty() {
        return Err(AppError::new("NoValidNodes", "配置中没有代理节点"));
    }
    if config.rules.is_empty() {
        return Err(AppError::new("NoRoutingRules", "配置中没有分流规则"));
    }
    let mut names: HashSet<&str> = BUILTIN_TARGETS.iter().copied().collect();
    for name in config
        .proxies
        .iter()
        .map(|node| node.name.as_str())
        .chain(config.proxy_groups.iter().map(|group| group.name.as_str()))
    {
        if name.trim().is_empty() || !names.insert(name) {
            return Err(AppError::new(
                "DuplicateName",
                "节点或策略组名称为空、重名或使用保留名称",
            ));
        }
    }
    let group_ids: HashMap<&str, usize> = config
        .proxy_groups
        .iter()
        .enumerate()
        .map(|(index, group)| (group.name.as_str(), index))
        .collect();
    let mut degrees = vec![0usize; config.proxy_groups.len()];
    let mut dependents = vec![Vec::new(); config.proxy_groups.len()];
    for (index, group) in config.proxy_groups.iter().enumerate() {
        validate_group_options(
            &group.group_type,
            group.url.as_deref(),
            group.interval,
            group.timeout,
        )?;
        if group.proxies.is_empty() {
            return Err(AppError::new(
                "EmptyProxyGroup",
                format!("策略组「{}」没有可用节点", group.name),
            ));
        }
        for candidate in &group.proxies {
            if !names.contains(candidate.as_str()) {
                return Err(AppError::new(
                    "InvalidGroupReference",
                    format!("策略组「{}」引用了不存在的节点或组", group.name),
                ));
            }
            if let Some(&dependency) = group_ids.get(candidate.as_str()) {
                degrees[index] += 1;
                dependents[dependency].push(index);
            }
        }
    }
    let mut ready: VecDeque<usize> = degrees
        .iter()
        .enumerate()
        .filter_map(|(index, &degree)| (degree == 0).then_some(index))
        .collect();
    let mut visited = 0;
    while let Some(index) = ready.pop_front() {
        visited += 1;
        for &dependent in &dependents[index] {
            degrees[dependent] -= 1;
            if degrees[dependent] == 0 {
                ready.push_back(dependent);
            }
        }
    }
    if visited != config.proxy_groups.len() {
        return Err(AppError::new("GroupCycle", "策略组存在循环引用"));
    }
    for (index, rule) in config.rules.iter().enumerate() {
        let target = super::rules::validate_rule(rule)?;
        if rule
            .split(',')
            .next()
            .is_some_and(|kind| kind.trim() == "MATCH")
            && index + 1 != config.rules.len()
        {
            return Err(AppError::new(
                "InvalidRuleOrder",
                "MATCH 兜底规则必须唯一并位于末尾",
            ));
        }
        if !names.contains(target) {
            return Err(AppError::new(
                "InvalidRuleTarget",
                "分流规则引用了不存在的策略",
            ));
        }
    }
    Ok(())
}
