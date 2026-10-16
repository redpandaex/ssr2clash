use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AppError {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PresetSource {
    #[default]
    Builtin,
    LocalIni {
        path: PathBuf,
    },
    RemoteIni {
        url: String,
    },
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FetchProxy {
    #[default]
    Direct,
    System,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    #[default]
    Live,
    Snapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileConfig {
    pub source_url: String,
    pub filter_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
    pub enable_udp: bool,
    pub preset: PresetSource,
}
impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            source_url: String::new(),
            filter_keywords: vec![],
            exclude_keywords: vec![],
            enable_udp: true,
            preset: PresetSource::Builtin,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub profile: ProfileConfig,
    pub port: u16,
    pub close_to_tray: bool,
    pub fetch_proxy: FetchProxy,
    pub theme: Theme,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            profile: ProfileConfig::default(),
            port: 25501,
            close_to_tray: true,
            fetch_proxy: FetchProxy::Direct,
            theme: Theme::System,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedConfig {
    pub config_revision: u64,
    pub yaml: String,
    pub node_count: usize,
    pub group_count: usize,
    #[serde(default)]
    pub invalid_count: usize,
    pub warnings: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDetails {
    pub yaml: String,
    pub nodes: Vec<super::clash::ClashProxy>,
    pub groups: Vec<super::clash::ClashProxyGroup>,
    pub decoded_preview: Option<String>,
}
