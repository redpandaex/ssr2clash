pub mod autostart;
pub mod lifecycle;
pub mod tray;

use crate::{
    conversion::ConversionEngine,
    model::app::{AppError, CoreStatus, RunMode},
    server::ServerController,
    service::AppService,
    storage::{detect_data_paths, DataPaths, FileStore},
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

pub struct CoreContext {
    pub service: Arc<AppService>,
    pub server: ServerController,
    pub paths: DataPaths,
}

pub struct DesktopContext {
    core: Result<CoreContext, AppError>,
    pub startup_error: Mutex<Option<String>>,
    pub quitting: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopView {
    pub run_mode: RunMode,
    pub data_directory: String,
    pub app_version: String,
    pub autostart_enabled: bool,
    pub autostart_error: Option<String>,
    pub startup_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    pub core: CoreStatus,
    pub desktop: DesktopView,
}

impl DesktopContext {
    pub async fn initialize(app: &AppHandle) -> Self {
        let core = async {
            let exe = std::env::current_exe()
                .map_err(|_| AppError::new("ExecutablePath", "无法确定程序路径"))?;
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|_| AppError::new("DataDirectory", "无法确定用户数据目录"))?;
            let paths = detect_data_paths(&exe, &app_data)?;
            let store = Arc::new(FileStore::new(paths.clone()));
            let service = Arc::new(
                AppService::open(store, Arc::new(ConversionEngine))
                    .await
                    .map_err(|mut error| {
                        error.message =
                            format!("{}；数据目录：{}", error.message, paths.root.display());
                        error
                    })?,
            );
            let server = ServerController::new(service.clone());
            Ok(CoreContext {
                service,
                server,
                paths,
            })
        }
        .await;
        Self {
            core,
            startup_error: Mutex::new(None),
            quitting: AtomicBool::new(false),
        }
    }

    pub fn core(&self) -> Result<&CoreContext, AppError> {
        self.core.as_ref().map_err(Clone::clone)
    }

    pub async fn view(&self, app: &AppHandle) -> Result<AppView, AppError> {
        let context = self.core()?;
        let (autostart_enabled, autostart_error) = match autostart::AutostartBackend::is_enabled(
            &autostart::SystemAutostart::new(app.clone()),
        ) {
            Ok(enabled) => (enabled, None),
            Err(error) => (false, Some(error.message)),
        };
        Ok(AppView {
            core: context.service.status().await,
            desktop: DesktopView {
                run_mode: context.paths.run_mode,
                data_directory: context.paths.root.display().to_string(),
                app_version: app.package_info().version.to_string(),
                autostart_enabled,
                autostart_error,
                startup_error: self.startup_error.lock().await.clone(),
            },
        })
    }
}

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn request_exit(app: &AppHandle) {
    let context = app.state::<DesktopContext>();
    if context.quitting.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(core) = app.state::<DesktopContext>().core() {
            let _ = core.server.shutdown().await;
            core.service.cancel_refreshes().await;
            let _ = core
                .service
                .wait_idle(std::time::Duration::from_secs(2))
                .await;
        }
        app.exit(0);
    });
}

pub fn attach_state_events(app: &AppHandle) {
    let Ok(core) = app
        .state::<DesktopContext>()
        .core()
        .map(|core| core.service.clone())
    else {
        return;
    };
    let mut events = core.subscribe();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while events.changed().await.is_ok() {
            let status = core.status().await;
            tray::update(&app, &status);
            let _ = app.emit("app-state-changed", ());
        }
    });
}
