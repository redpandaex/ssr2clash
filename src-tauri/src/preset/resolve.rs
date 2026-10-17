use super::{
    ini::{at_line, parse_bool, parse_ini},
    limits::{check_rule_limits, GroupBudget},
    load_builtin_preset,
    model::{GroupDefinition, ResolvedPreset},
    rules::{convert_rule, merge_rules, parse_rule_list},
};
use crate::{
    model::{
        app::{AppError, PresetSource},
        clash::ClashProxyGroup,
    },
    subscription::http::{parse_http_url, HttpFetcher, MAX_TEXT_BYTES},
};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};
use tokio::io::AsyncReadExt;
use url::Url;

const MAX_IMPORTED_BYTES: usize = 16 * 1024 * 1024;
type ImportFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

#[derive(Clone)]
enum Location {
    Local(PathBuf),
    Remote(Url),
}
impl Location {
    fn key(&self) -> String {
        match self {
            Self::Local(path) => format!("file:{}", path.display()),
            Self::Remote(url) => format!("url:{url}"),
        }
    }
}

struct Context<'a> {
    fetcher: &'a HttpFetcher,
    root: Option<PathBuf>,
    cache: HashMap<String, Arc<str>>,
    stack: HashSet<String>,
    bytes: usize,
    rule_bytes: usize,
    rule_count: usize,
    group_budget: GroupBudget,
}

impl Context<'_> {
    async fn reference(&self, base: &Location, value: &str) -> Result<Location, AppError> {
        match base {
            Location::Remote(url) => {
                let mut url = url
                    .join(value)
                    .map_err(|_| AppError::new("UnsupportedResource", "资源引用地址无效"))?;
                parse_http_url(url.as_str())?;
                url.set_fragment(None);
                Ok(Location::Remote(url))
            }
            Location::Local(path) => {
                if value.starts_with("http://") || value.starts_with("https://") {
                    return Ok(Location::Remote(parse_http_url(value)?));
                }
                if value.contains("://") {
                    return Err(AppError::new("UnsupportedResource", "不支持该资源引用方式"));
                }
                let candidate = PathBuf::from(value);
                let candidate = if candidate.is_absolute() {
                    candidate
                } else {
                    path.parent().unwrap().join(candidate)
                };
                let canonical = tokio::fs::canonicalize(candidate)
                    .await
                    .map_err(|_| AppError::new("ResourceRead", "无法读取引用的本地文件"))?;
                if !self
                    .root
                    .as_ref()
                    .is_some_and(|root| canonical.starts_with(root))
                {
                    return Err(AppError::new(
                        "ResourceOutsideRoot",
                        "本地资源超出所选配置目录",
                    ));
                }
                Ok(Location::Local(canonical))
            }
        }
    }

    async fn load(&mut self, location: &Location) -> Result<Arc<str>, AppError> {
        let key = location.key();
        if let Some(text) = self.cache.get(&key) {
            return Ok(text.clone());
        }
        let text = match location {
            Location::Remote(url) => self.fetcher.get_text(url.as_str()).await?,
            Location::Local(path) => {
                let file = tokio::fs::File::open(path)
                    .await
                    .map_err(|_| AppError::new("ResourceRead", "无法打开本地配置或规则"))?;
                let mut bytes = Vec::new();
                file.take(MAX_TEXT_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|_| AppError::new("ResourceRead", "无法读取本地资源"))?;
                if bytes.len() > MAX_TEXT_BYTES {
                    return Err(AppError::new("ResponseTooLarge", "本地资源超过 8 MiB 限制"));
                }
                String::from_utf8(bytes)
                    .map_err(|_| AppError::new("InvalidUtf8", "本地资源不是有效 UTF-8 文本"))?
            }
        };
        self.bytes = self.bytes.saturating_add(text.len());
        if self.bytes > MAX_IMPORTED_BYTES {
            return Err(AppError::new(
                "ImportTooLarge",
                "导入资源总量超过 16 MiB 限制",
            ));
        }
        let text: Arc<str> = text.into();
        self.cache.insert(key, text.clone());
        Ok(text)
    }

    fn enter(&mut self, location: &Location, depth: usize) -> Result<String, AppError> {
        if depth > 8 {
            return Err(AppError::new("ImportDepth", "导入嵌套超过 8 层"));
        }
        let key = location.key();
        if !self.stack.insert(key.clone()) {
            return Err(AppError::new("ImportCycle", "配置导入存在循环引用"));
        }
        Ok(key)
    }

    fn groups<'a>(
        &'a mut self,
        value: String,
        base: Location,
        depth: usize,
    ) -> ImportFuture<'a, Vec<GroupDefinition>> {
        Box::pin(async move {
            if let Some(reference) = value.strip_prefix("!!import:") {
                let location = self.reference(&base, reference.trim()).await?;
                let key = self.enter(&location, depth)?;
                let text = self.load(&location).await?;
                let mut groups = Vec::new();
                for (index, line) in meaningful_lines(&text) {
                    let line = line.strip_prefix("custom_proxy_group=").unwrap_or(line);
                    groups.extend(
                        self.groups(line.into(), location.clone(), depth + 1)
                            .await
                            .map_err(|error| at_line(error, index))?,
                    );
                }
                self.stack.remove(&key);
                Ok(groups)
            } else {
                let definition = GroupDefinition::parse(&value)?;
                self.group_budget.add(&definition)?;
                for selector in &definition.selectors {
                    if selector.starts_with("[]") {
                        continue;
                    }
                    let patterns: Vec<&str> = if let Some(value) = selector.strip_prefix("!!GROUP=")
                    {
                        value.splitn(2, "!!").collect()
                    } else if selector.starts_with("!!") {
                        return Err(AppError::new(
                            "UnsupportedPresetOption",
                            "不支持该节点选择条件",
                        ));
                    } else {
                        vec![selector.as_str()]
                    };
                    for pattern in patterns {
                        fancy_regex::Regex::new(pattern)
                            .map_err(|_| AppError::new("InvalidRegex", "分组中的正则表达式无效"))?;
                    }
                }
                Ok(vec![definition])
            }
        })
    }

    fn rulesets<'a>(
        &'a mut self,
        value: String,
        base: Location,
        depth: usize,
    ) -> ImportFuture<'a, Vec<String>> {
        Box::pin(async move {
            if let Some(reference) = value.strip_prefix("!!import:") {
                let location = self.reference(&base, reference.trim()).await?;
                let key = self.enter(&location, depth)?;
                let text = self.load(&location).await?;
                let mut rules = Vec::new();
                for (index, line) in meaningful_lines(&text) {
                    let line = line.strip_prefix("ruleset=").unwrap_or(line);
                    rules.extend(
                        self.rulesets(line.into(), location.clone(), depth + 1)
                            .await
                            .map_err(|error| at_line(error, index))?,
                    );
                }
                self.stack.remove(&key);
                return Ok(rules);
            }
            let (target, reference) = value
                .split_once(',')
                .ok_or_else(|| AppError::new("InvalidRule", "规则源必须指定策略组与资源"))?;
            let target = target.trim();
            let reference = reference.trim();
            if target.is_empty() || target.len() > 1024 {
                return Err(AppError::new("InvalidRule", "规则策略名称无效"));
            }
            let rules = if let Some(rule) = reference.strip_prefix("[]") {
                convert_rule(rule, target)?.into_iter().collect()
            } else {
                let reference = reference
                    .rsplit_once(',')
                    .filter(|(_, interval)| interval.parse::<u64>().is_ok())
                    .map(|(value, _)| value)
                    .unwrap_or(reference);
                let (format, reference) = [
                    "surge",
                    "quanx",
                    "clash-classic",
                    "clash-domain",
                    "clash-ipcidr",
                ]
                .iter()
                .find_map(|format| {
                    reference
                        .strip_prefix(&format!("{format}:"))
                        .map(|value| (*format, value))
                })
                .unwrap_or(("surge", reference));
                let location = self.reference(&base, reference).await?;
                let text = self.load(&location).await?;
                parse_rule_list(&text, target, format)?
            };
            for rule in &rules {
                self.rule_bytes = self.rule_bytes.saturating_add(rule.len());
            }
            self.rule_count += rules.len();
            if self.rule_bytes > MAX_IMPORTED_BYTES || self.rule_count > 100_000 {
                return Err(AppError::new("ImportTooLarge", "展开的规则超过大小限制"));
            }
            Ok(rules)
        })
    }
}

fn meaningful_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| {
            !line.is_empty() && !line.starts_with(['#', ';']) && !line.starts_with("//")
        })
}

pub async fn resolve_preset(
    source: &PresetSource,
    fetcher: &HttpFetcher,
) -> Result<ResolvedPreset, AppError> {
    if matches!(source, PresetSource::Builtin) {
        return load_builtin_preset();
    }
    let location = match source {
        PresetSource::LocalIni { path } => Location::Local(
            tokio::fs::canonicalize(path)
                .await
                .map_err(|_| AppError::new("ResourceRead", "找不到所选 INI 配置文件"))?,
        ),
        PresetSource::RemoteIni { url } => Location::Remote(parse_http_url(url)?),
        PresetSource::Builtin => unreachable!(),
    };
    let root = match &location {
        Location::Local(path) => path.parent().map(PathBuf::from),
        _ => None,
    };
    let mut context = Context {
        fetcher,
        root,
        cache: HashMap::new(),
        stack: HashSet::new(),
        bytes: 0,
        rule_bytes: 0,
        rule_count: 0,
        group_budget: GroupBudget::default(),
    };
    context.enter(&location, 0)?;
    let text = context.load(&location).await?;
    let entries = parse_ini(&text)?;
    let mut enabled = true;
    let mut overwrite = false;
    let mut base = serde_json::Map::new();
    for entry in &entries {
        match entry.key.as_str() {
            "enable_rule_generator" => enabled = parse_bool(&entry.value, entry.line)?,
            "overwrite_original_rules" => overwrite = parse_bool(&entry.value, entry.line)?,
            "clash_rule_base" => {
                let resource = context
                    .reference(&location, &entry.value)
                    .await
                    .map_err(|error| at_line(error, entry.line))?;
                let text = context
                    .load(&resource)
                    .await
                    .map_err(|error| at_line(error, entry.line))?;
                let document: serde_json::Value = serde_saphyr::from_str(&text).map_err(|_| {
                    at_line(
                        AppError::new(
                            "InvalidBaseConfig",
                            "基础配置必须是普通 YAML，模板语法不执行",
                        ),
                        entry.line,
                    )
                })?;
                base = document
                    .as_object()
                    .cloned()
                    .ok_or_else(|| AppError::new("InvalidBaseConfig", "基础 YAML 必须是对象"))?;
            }
            _ => {}
        }
    }
    let base_rules = match base.remove("rules") {
        Some(value) => serde_json::from_value::<Vec<String>>(value)
            .map_err(|_| AppError::new("InvalidBaseConfig", "基础配置的 rules 必须是文本数组"))?,
        None => vec![],
    };
    let base_groups = base.remove("proxy-groups");
    base.remove("proxies");
    let mut groups = Vec::new();
    let mut generated = Vec::new();
    let mut had_ruleset = false;
    for entry in &entries {
        match entry.key.as_str() {
            "custom_proxy_group" => groups.extend(
                context
                    .groups(entry.value.clone(), location.clone(), 1)
                    .await
                    .map_err(|error| at_line(error, entry.line))?,
            ),
            "ruleset" if enabled => {
                had_ruleset = true;
                generated.extend(
                    context
                        .rulesets(entry.value.clone(), location.clone(), 1)
                        .await
                        .map_err(|error| at_line(error, entry.line))?,
                );
            }
            _ => {}
        }
    }
    if groups.is_empty() {
        if let Some(value) = base_groups {
            let base_groups: Vec<ClashProxyGroup> = serde_json::from_value(value)
                .map_err(|_| AppError::new("InvalidBaseConfig", "基础配置的策略组无效"))?;
            groups = base_groups
                .into_iter()
                .map(|group| GroupDefinition {
                    name: group.name,
                    kind: group.group_type,
                    selectors: group
                        .proxies
                        .into_iter()
                        .map(|value| format!("[]{value}"))
                        .collect(),
                    url: group.url,
                    interval: group.interval,
                    timeout: group.timeout,
                    tolerance: group.tolerance,
                })
                .collect();
            for group in &groups {
                context.group_budget.add(group)?;
            }
        } else {
            groups = load_builtin_preset()?.groups;
        }
    }
    if enabled && !had_ruleset && base_rules.is_empty() {
        generated = load_builtin_preset()?.rules;
    }
    let original = if !enabled || !overwrite {
        base_rules
    } else {
        vec![]
    };
    let rules = merge_rules(original, if enabled { generated } else { vec![] })?;
    check_rule_limits(&rules)?;
    if rules.is_empty() {
        return Err(AppError::new(
            "NoRoutingRules",
            "配置没有可用规则，请启用规则生成或提供包含 rules 的基础 YAML",
        ));
    }
    Ok(ResolvedPreset {
        groups,
        rules,
        base,
    })
}
