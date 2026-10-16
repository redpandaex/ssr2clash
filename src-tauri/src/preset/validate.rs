use super::BUILTIN_TARGETS;
use crate::model::{app::AppError, clash::ClashConfig};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn validate_config(config: &ClashConfig) -> Result<(), AppError> {
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
        if name.is_empty() || !names.insert(name) {
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
    for rule in &config.rules {
        let fields: Vec<&str> = rule.split(',').map(str::trim).collect();
        let target = if fields.last() == Some(&"no-resolve") {
            fields.get(fields.len().saturating_sub(2))
        } else {
            fields.last()
        };
        if fields.len() < 2 || !target.is_some_and(|target| names.contains(target)) {
            return Err(AppError::new(
                "InvalidRuleTarget",
                "分流规则引用了不存在的策略",
            ));
        }
    }
    Ok(())
}
