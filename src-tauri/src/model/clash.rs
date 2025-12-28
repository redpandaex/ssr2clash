use serde::{Deserialize, Serialize};

/// Clash 代理节点结构
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs_param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_param: Option<String>,
}

/// Clash 完整配置
#[derive(Debug, Serialize, Deserialize)]
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
}
