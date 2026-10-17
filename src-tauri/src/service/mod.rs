mod refresh;
mod snapshot;

use crate::{
    conversion::{now_rfc3339, Converter},
    model::{
        app::{
            AppError, AppSettings, CoreStatus, GeneratedConfig, OutputDetails, OutputMode,
            RuntimeLogEntry, SavedState,
        },
        clash::ClashConfig,
    },
    network,
    storage::Storage,
    subscription::http::parse_http_url,
};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::sync::{watch, Mutex};

#[derive(Clone)]
pub struct AppService {
    inner: Arc<Inner>,
}
struct Inner {
    runtime: Mutex<Runtime>,
    store: Arc<dyn Storage>,
    converter: Arc<dyn Converter>,
    events: watch::Sender<u64>,
}

#[derive(Clone)]
struct Runtime {
    saved: SavedState,
    epoch: u64,
    next_request: u64,
    last_committed_request: u64,
    in_flight: usize,
    bound_port: Option<u16>,
    details: Option<OutputDetails>,
    last_request_at: Option<String>,
    last_error: Option<String>,
    served_stale: bool,
    logs: VecDeque<RuntimeLogEntry>,
    log_sequence: u64,
}
impl Runtime {
    fn log(&mut self, level: &str, message: impl Into<String>) {
        self.log_sequence = self.log_sequence.saturating_add(1);
        if self.logs.len() == 100 {
            self.logs.pop_front();
        }
        self.logs.push_back(RuntimeLogEntry {
            id: self.log_sequence,
            at: now_rfc3339(),
            level: level.into(),
            message: message.into(),
        });
    }
}

#[derive(Debug)]
pub struct ServedYaml {
    pub yaml: String,
    pub stale: bool,
}

fn active(runtime: &Runtime) -> Option<&GeneratedConfig> {
    let result = match runtime.saved.mode {
        OutputMode::Live => &runtime.saved.last_good,
        OutputMode::Snapshot => &runtime.saved.frozen,
    };
    result
        .as_ref()
        .filter(|output| output.config_revision == runtime.saved.revision)
}

fn status_of(runtime: &Runtime) -> CoreStatus {
    let subscription_url = if runtime.saved.settings.profile.source_url.is_empty() {
        None
    } else {
        let host = if runtime.saved.settings.allow_lan {
            network::lan_ipv4_address()
        } else {
            Some(std::net::Ipv4Addr::LOCALHOST)
        };
        host.map(|host| {
            format!(
                "http://{host}:{}/sub/{}.yaml",
                runtime.bound_port.unwrap_or(runtime.saved.settings.port),
                runtime.saved.subscription_token
            )
        })
    };
    CoreStatus {
        settings: runtime.saved.settings.clone(),
        revision: runtime.saved.revision,
        mode: runtime.saved.mode,
        server_running: runtime.bound_port.is_some(),
        refreshing: runtime.in_flight > 0,
        subscription_url,
        active_result: active(runtime).cloned(),
        last_request_at: runtime.last_request_at.clone(),
        last_error: runtime.last_error.clone(),
        served_stale: runtime.served_stale,
        logs: runtime.logs.iter().cloned().collect(),
    }
}

fn validate_source(settings: &AppSettings, required: bool) -> Result<(), AppError> {
    if settings.port == 0 {
        return Err(AppError::new(
            "InvalidPort",
            "服务端口必须在 1 到 65535 之间",
        ));
    }
    if settings.profile.source_url.trim().is_empty() {
        return if required {
            Err(AppError::new("InvalidSourceUrl", "请先保存原订阅地址"))
        } else {
            Ok(())
        };
    }
    let url = parse_http_url(&settings.profile.source_url)?;
    if url.host_str().is_some_and(network::is_local_host)
        && url.port_or_known_default() == Some(settings.port)
        && url.path().starts_with("/sub/")
    {
        return Err(AppError::new(
            "SourceLoop",
            "请填写原始订阅地址，不要填写本软件的本机或局域网订阅链接",
        ));
    }
    Ok(())
}

impl AppService {
    pub async fn open(
        store: Arc<dyn Storage>,
        converter: Arc<dyn Converter>,
    ) -> Result<Self, AppError> {
        let saved = match store.load().await? {
            Some(saved) => saved,
            None => {
                let saved = SavedState::fresh();
                store.save(&saved).await?;
                saved
            }
        };
        let (events, _) = watch::channel(0);
        Ok(Self {
            inner: Arc::new(Inner {
                store,
                converter,
                events,
                runtime: Mutex::new(Runtime {
                    saved,
                    epoch: 0,
                    next_request: 0,
                    last_committed_request: 0,
                    in_flight: 0,
                    bound_port: None,
                    details: None,
                    last_request_at: None,
                    last_error: None,
                    served_stale: false,
                    logs: VecDeque::new(),
                    log_sequence: 0,
                }),
            }),
        })
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.events.subscribe()
    }
    fn publish(&self, runtime: &Runtime) {
        self.inner.events.send_replace(runtime.epoch);
    }
    pub async fn status(&self) -> CoreStatus {
        let runtime = self.inner.runtime.lock().await;
        status_of(&runtime)
    }
    pub async fn should_restore_service(&self) -> bool {
        self.inner.runtime.lock().await.saved.service_enabled
    }
    pub async fn subscription_token(&self) -> String {
        self.inner
            .runtime
            .lock()
            .await
            .saved
            .subscription_token
            .clone()
    }

    async fn transact<F>(&self, change: F) -> Result<CoreStatus, AppError>
    where
        F: FnOnce(&mut Runtime) -> Result<bool, AppError> + Send + 'static,
    {
        let service = self.clone();
        tokio::spawn(async move {
            let mut current = service.inner.runtime.lock().await;
            let mut candidate = current.clone();
            if !change(&mut candidate)? {
                return Ok(status_of(&current));
            }
            service.inner.store.save(&candidate.saved).await?;
            *current = candidate;
            service.publish(&current);
            Ok(status_of(&current))
        })
        .await
        .map_err(|_| AppError::new("InternalTask", "状态保存任务未能完成"))?
    }

    pub async fn save_settings(&self, mut settings: AppSettings) -> Result<CoreStatus, AppError> {
        settings.profile.source_url = settings.profile.source_url.trim().to_owned();
        self.transact(move |runtime| {
            let profile_changed = runtime.saved.settings.profile != settings.profile;
            if profile_changed && runtime.saved.mode == OutputMode::Snapshot {
                return Err(AppError::new(
                    "SnapshotFrozen",
                    "请先关闭快照模式再修改输出配置",
                ));
            }
            if runtime.bound_port.is_some()
                && (runtime.saved.settings.port != settings.port
                    || runtime.saved.settings.allow_lan != settings.allow_lan)
            {
                return Err(AppError::new(
                    "ServerMustStop",
                    "请先停止本地服务再修改端口或局域网访问设置",
                ));
            }
            validate_source(&settings, runtime.bound_port.is_some())?;
            if settings == runtime.saved.settings {
                return Ok(false);
            }
            if profile_changed {
                runtime.saved.revision = runtime
                    .saved
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::new("RevisionLimit", "配置版本已超出范围"))?;
                runtime.saved.last_good = None;
                runtime.saved.frozen = None;
                runtime.details = None;
                runtime.last_error = None;
                runtime.served_stale = false;
            }
            if profile_changed || runtime.saved.settings.fetch_proxy != settings.fetch_proxy {
                runtime.epoch = runtime.epoch.wrapping_add(1);
            }
            if settings.profile.source_url.is_empty() {
                runtime.saved.service_enabled = false;
            }
            runtime.saved.settings = settings;
            runtime.log("info", "已保存设置");
            Ok(true)
        })
        .await
    }

    pub async fn set_service_enabled(&self, enabled: bool) -> Result<CoreStatus, AppError> {
        self.transact(move |runtime| {
            if enabled {
                validate_source(&runtime.saved.settings, true)?;
            }
            if runtime.saved.service_enabled == enabled {
                return Ok(false);
            }
            runtime.saved.service_enabled = enabled;
            Ok(true)
        })
        .await
    }

    /// 只有监听控制器在绑定／释放真实端口后调用。
    pub async fn mark_server_running(&self, port: Option<u16>) {
        let mut runtime = self.inner.runtime.lock().await;
        runtime.bound_port = port;
        runtime.log(
            "info",
            if port.is_some() {
                "本地订阅服务已启动"
            } else {
                "本地订阅服务已停止"
            },
        );
        self.publish(&runtime);
    }

    pub async fn cancel_refreshes(&self) {
        let mut runtime = self.inner.runtime.lock().await;
        runtime.epoch = runtime.epoch.wrapping_add(1);
        if runtime.in_flight > 0 {
            runtime.log("info", "已取消执行中的更新");
        }
        self.publish(&runtime);
    }

    pub async fn wait_idle(&self, duration: Duration) -> bool {
        let mut events = self.subscribe();
        tokio::time::timeout(duration, async {
            loop {
                if self.inner.runtime.lock().await.in_flight == 0 {
                    break;
                }
                if events.changed().await.is_err() {
                    break;
                }
            }
        })
        .await
        .is_ok()
    }

    pub async fn output_details(&self) -> Result<OutputDetails, AppError> {
        let runtime = self.inner.runtime.lock().await;
        let output = active(&runtime).ok_or_else(|| AppError::new("NoOutput", "请先生成配置"))?;
        if let Some(details) = runtime
            .details
            .as_ref()
            .filter(|details| details.yaml == output.yaml)
        {
            return Ok(details.clone());
        }
        let config: ClashConfig = serde_saphyr::from_str(&output.yaml)
            .map_err(|_| AppError::new("StateCorrupted", "已保存的配置无法解析"))?;
        Ok(OutputDetails {
            yaml: output.yaml.clone(),
            nodes: config.proxies,
            groups: config.proxy_groups,
            decoded_preview: None,
        })
    }
}
