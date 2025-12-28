use serde::Deserialize;

/// 转换配置
#[derive(Debug, Clone, Deserialize)]
pub struct ConvertConfig {
    #[serde(rename = "filterKeywords")]
    pub filter_keywords: Vec<String>,
    #[serde(rename = "excludeKeywords")]
    pub exclude_keywords: Vec<String>,
    #[serde(rename = "renameRules")]
    pub rename_rules: Vec<RenameRule>,
    #[serde(rename = "sortBy")]
    pub sort_by: String,
    #[serde(rename = "enableUdp")]
    pub enable_udp: bool,
}

/// 重命名规则
#[derive(Debug, Clone, Deserialize)]
pub struct RenameRule {
    pub pattern: String,
    pub replacement: String,
}
