use serde::{Deserialize, Serialize};

/// SSR 节点结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SsrNode {
    pub server: String,
    pub server_port: u16,
    pub protocol: String,
    pub method: String,
    pub obfs: String,
    pub password: String,
    #[serde(default)]
    pub obfs_param: String,
    #[serde(default)]
    pub protocol_param: String,
    #[serde(default)]
    pub remarks: String,
    #[serde(default)]
    pub group: String,
}
