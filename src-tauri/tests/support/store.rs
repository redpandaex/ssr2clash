use async_trait::async_trait;
use ssr2clash_lib::{
    conversion::{compile_subscription, ConversionResult, Converter},
    model::app::{AppError, FetchProxy, ProfileConfig, SavedState},
    preset::load_builtin_preset,
    storage::Storage,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Gate {
    pub entered: Arc<Semaphore>,
    pub release: Arc<Semaphore>,
}
impl Gate {
    fn new() -> Self {
        Self {
            entered: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
        }
    }
    pub async fn wait(&self) {
        tokio::time::timeout(std::time::Duration::from_secs(5), self.entered.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    }
    pub fn open(&self) {
        self.release.add_permits(1);
    }
    async fn pause(&self) {
        self.entered.add_permits(1);
        self.release.acquire().await.unwrap().forget();
    }
}

pub struct MemoryStore {
    state: Mutex<Option<SavedState>>,
    failure: AtomicBool,
    gate: Mutex<Option<Gate>>,
}
impl MemoryStore {
    pub fn new(state: Option<SavedState>) -> Self {
        Self {
            state: Mutex::new(state),
            failure: AtomicBool::new(false),
            gate: Mutex::new(None),
        }
    }
    pub fn saved(&self) -> Option<SavedState> {
        self.state.lock().unwrap().clone()
    }
    pub fn fail_next(&self) {
        self.failure.store(true, Ordering::SeqCst);
    }
    pub fn block_next(&self) -> Gate {
        let gate = Gate::new();
        *self.gate.lock().unwrap() = Some(gate.clone());
        gate
    }
}
#[async_trait]
impl Storage for MemoryStore {
    async fn load(&self) -> Result<Option<SavedState>, AppError> {
        Ok(self.saved())
    }
    async fn save(&self, state: &SavedState) -> Result<(), AppError> {
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.pause().await;
        }
        if self.failure.swap(false, Ordering::SeqCst) {
            return Err(AppError::new("StorageWrite", "模拟磁盘写入失败"));
        }
        *self.state.lock().unwrap() = Some(state.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct TestConverter {
    calls: AtomicUsize,
    failure: Mutex<Option<AppError>>,
    gate: Mutex<Option<Gate>>,
}
impl TestConverter {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    pub fn fail_next(&self) {
        *self.failure.lock().unwrap() =
            Some(AppError::new("HttpStatus", "资源请求失败（HTTP 503）"));
    }
    pub fn block_next(&self) -> Gate {
        let gate = Gate::new();
        *self.gate.lock().unwrap() = Some(gate.clone());
        gate
    }
}
#[async_trait]
impl Converter for TestConverter {
    async fn generate(
        &self,
        profile: ProfileConfig,
        revision: u64,
        proxy: FetchProxy,
    ) -> Result<ConversionResult, AppError> {
        let sequence = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let failure = self.failure.lock().unwrap().take();
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.pause().await;
        }
        if let Some(error) = failure {
            return Err(error);
        }
        let prefix = if profile.source_url.contains("/new") {
            "new"
        } else if proxy == FetchProxy::System {
            "system"
        } else {
            "direct"
        };
        let raw = super::ssr_link(
            &format!("{prefix}-{sequence}.example.invalid"),
            "香港 01",
            "测试",
            false,
        );
        compile_subscription(&raw, &profile, &load_builtin_preset()?, revision)
    }
}
