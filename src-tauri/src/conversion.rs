use crate::{
    model::app::{AppError, FetchProxy, GeneratedConfig, OutputDetails, ProfileConfig},
    preset::{model::ResolvedPreset, resolve::resolve_preset},
    ssr::{converter::build_clash_config, filter::filter_nodes},
    subscription::{http::HttpFetcher, parse_subscription, ParsedSubscription},
};
use async_trait::async_trait;
use chrono::{SecondsFormat, Utc};

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub output: GeneratedConfig,
    pub details: OutputDetails,
}

#[async_trait]
pub trait Converter: Send + Sync {
    async fn generate(
        &self,
        profile: ProfileConfig,
        revision: u64,
        fetch_proxy: FetchProxy,
    ) -> Result<ConversionResult, AppError>;
}

#[derive(Debug, Clone, Default)]
pub struct ConversionEngine;

#[async_trait]
impl Converter for ConversionEngine {
    async fn generate(
        &self,
        profile: ProfileConfig,
        revision: u64,
        fetch_proxy: FetchProxy,
    ) -> Result<ConversionResult, AppError> {
        let fetcher = HttpFetcher::new(fetch_proxy)?;
        let raw = fetcher.get_text(&profile.source_url).await?;
        let parsed = parse_and_filter(&raw, &profile)?;
        let preset = resolve_preset(&profile.preset, &fetcher).await?;
        finish_conversion(parsed, &profile, &preset, revision)
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn compile_subscription(
    raw: &str,
    profile: &ProfileConfig,
    preset: &ResolvedPreset,
    revision: u64,
) -> Result<ConversionResult, AppError> {
    finish_conversion(parse_and_filter(raw, profile)?, profile, preset, revision)
}

fn parse_and_filter(raw: &str, profile: &ProfileConfig) -> Result<ParsedSubscription, AppError> {
    let mut parsed = parse_subscription(raw)?;
    let before = parsed.nodes.len();
    parsed.nodes = filter_nodes(
        parsed.nodes,
        &profile.filter_keywords,
        &profile.exclude_keywords,
    );
    if parsed.nodes.is_empty() {
        return Err(AppError::new(
            "EmptyFilter",
            "过滤后没有剩余节点，请调整包含或排除关键词",
        ));
    }
    if parsed.nodes.len() != before {
        parsed.warnings.push(format!(
            "已按关键词过滤 {} 个节点",
            before - parsed.nodes.len()
        ));
    }
    Ok(parsed)
}

fn finish_conversion(
    parsed: ParsedSubscription,
    profile: &ProfileConfig,
    preset: &ResolvedPreset,
    revision: u64,
) -> Result<ConversionResult, AppError> {
    let config = build_clash_config(&parsed.nodes, preset, profile.enable_udp)?;
    let yaml = serde_saphyr::to_string(&config)
        .map_err(|_| AppError::new("YamlSerialization", "生成 YAML 配置失败"))?;
    let output = GeneratedConfig {
        config_revision: revision,
        yaml: yaml.clone(),
        node_count: config.proxies.len(),
        group_count: config.proxy_groups.len(),
        invalid_count: parsed.invalid_count,
        warnings: parsed.warnings,
        generated_at: now_rfc3339(),
    };
    let details = OutputDetails {
        yaml,
        nodes: config.proxies,
        groups: config.proxy_groups,
        decoded_preview: Some(parsed.decoded_preview),
    };
    Ok(ConversionResult { output, details })
}
