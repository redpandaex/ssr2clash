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
    pub allow_lan: bool,
    pub close_to_tray: bool,
    pub fetch_proxy: FetchProxy,
    pub theme: Theme,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            profile: ProfileConfig::default(),
            port: 25501,
            allow_lan: false,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunMode {
    Portable,
    Installed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedState {
    pub schema_version: u32,
    pub revision: u64,
    pub subscription_token: String,
    pub settings: AppSettings,
    pub mode: OutputMode,
    pub service_enabled: bool,
    pub last_good: Option<GeneratedConfig>,
    pub frozen: Option<GeneratedConfig>,
}

impl SavedState {
    pub fn fresh() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            subscription_token: uuid::Uuid::new_v4().simple().to_string(),
            settings: AppSettings::default(),
            mode: OutputMode::Live,
            service_enabled: false,
            last_good: None,
            frozen: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogEntry {
    pub id: u64,
    pub at: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreStatus {
    pub settings: AppSettings,
    pub revision: u64,
    pub mode: OutputMode,
    pub server_running: bool,
    pub refreshing: bool,
    pub subscription_url: Option<String>,
    pub active_result: Option<GeneratedConfig>,
    pub last_request_at: Option<String>,
    pub last_error: Option<String>,
    pub served_stale: bool,
    pub logs: Vec<RuntimeLogEntry>,
}
