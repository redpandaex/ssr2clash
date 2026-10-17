use serde::{Deserialize, Serialize};

/// Clash 代理节点结构
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClashProxy {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub server: String,
    pub port: u16,
    pub cipher: String,
    pub password: String,
    pub protocol: String,
    pub obfs: String,
    #[serde(rename = "obfs-param", skip_serializing_if = "Option::is_none")]
    pub obfs_param: Option<String>,
    #[serde(rename = "protocol-param", skip_serializing_if = "Option::is_none")]
    pub protocol_param: Option<String>,
    pub udp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClashProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u64>,
}

/// Clash 完整配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClashConfig {
    pub port: u16,
    #[serde(rename = "socks-port")]
    pub socks_port: u16,
    #[serde(rename = "allow-lan")]
    pub allow_lan: bool,
    pub mode: String,
    #[serde(rename = "log-level")]
    pub log_level: String,
    pub proxies: Vec<ClashProxy>,
    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ClashProxyGroup>,
    pub rules: Vec<String>,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
