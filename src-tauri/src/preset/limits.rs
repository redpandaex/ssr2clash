use super::{model::GroupDefinition, validate::validate_group_options, BUILTIN_TARGETS};
use crate::model::app::AppError;
use std::collections::HashSet;

pub const MAX_GROUPS: usize = 512;
pub const MAX_SELECTORS: usize = 8192;
pub const MAX_GROUP_DEFINITION_BYTES: usize = 1024 * 1024;
pub const MAX_MEMBER_REFERENCES: usize = 250_000;
pub const MAX_MEMBER_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MATCH_CHECKS: usize = 10_000_000;
pub const MAX_RULES: usize = 100_000;
pub const MAX_RULE_BYTES: usize = 16 * 1024 * 1024;

fn exceeded() -> AppError {
    AppError::new(
        "ImportTooLarge",
        "展开的分组、选择条件或节点成员超过资源限制，请缩小预设或筛选节点",
    )
}

#[derive(Default)]
pub(crate) struct GroupBudget {
    names: HashSet<String>,
    selectors: usize,
    bytes: usize,
}
impl GroupBudget {
    pub fn add(&mut self, group: &GroupDefinition) -> Result<(), AppError> {
        if group.name.trim().is_empty()
            || BUILTIN_TARGETS.contains(&group.name.as_str())
            || self.names.contains(&group.name)
        {
            return Err(AppError::new(
                "DuplicateName",
                "策略组名称为空、重名或使用保留名称",
            ));
        }
        validate_group_options(
            &group.kind,
            group.url.as_deref(),
            group.interval,
            group.timeout,
        )?;
        self.selectors = self.selectors.saturating_add(group.selectors.len());
        self.bytes = self
            .bytes
            .saturating_add(group.name.len())
            .saturating_add(group.kind.len())
            .saturating_add(group.url.as_ref().map_or(0, String::len));
        for selector in &group.selectors {
            self.bytes = self.bytes.saturating_add(selector.len());
        }
        if self.names.len() >= MAX_GROUPS
            || self.selectors > MAX_SELECTORS
            || self.bytes > MAX_GROUP_DEFINITION_BYTES
        {
            return Err(exceeded());
        }
        self.names.insert(group.name.clone());
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct MemberBudget {
    references: usize,
    bytes: usize,
}
impl MemberBudget {
    pub fn add(&mut self, name: &str) -> Result<(), AppError> {
        self.references = self.references.saturating_add(1);
        self.bytes = self.bytes.saturating_add(name.len());
        if self.references > MAX_MEMBER_REFERENCES || self.bytes > MAX_MEMBER_BYTES {
            return Err(exceeded());
        }
        Ok(())
    }
}

pub(crate) fn check_group_count(count: usize) -> Result<(), AppError> {
    if count > MAX_GROUPS {
        return Err(exceeded());
    }
    Ok(())
}

pub(crate) fn check_rule_limits(rules: &[String]) -> Result<(), AppError> {
    let bytes = rules
        .iter()
        .fold(0usize, |sum, rule| sum.saturating_add(rule.len()));
    if rules.len() > MAX_RULES || bytes > MAX_RULE_BYTES {
        return Err(AppError::new("ImportTooLarge", "展开的规则超过大小限制"));
    }
    Ok(())
}

pub(crate) fn reserve_checks(used: &mut usize, count: usize) -> Result<(), AppError> {
    *used = used.saturating_add(count);
    if *used > MAX_MATCH_CHECKS {
        return Err(exceeded());
    }
    Ok(())
}
