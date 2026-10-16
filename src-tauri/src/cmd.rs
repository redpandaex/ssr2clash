use crate::{
    desktop::{self, autostart, AppView, DesktopContext},
    model::app::{AppError, AppSettings, OutputDetails},
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_state(
    app: AppHandle,
    context: State<'_, DesktopContext>,
) -> Result<AppView, AppError> {
    context.view(&app).await
}

#[tauri::command]
pub async fn get_output_details(
    context: State<'_, DesktopContext>,
) -> Result<OutputDetails, AppError> {
    context.core()?.service.output_details().await
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    context: State<'_, DesktopContext>,
    settings: AppSettings,
) -> Result<AppView, AppError> {
    let theme = settings.theme;
    context.core()?.service.save_settings(settings).await?;
    desktop::apply_theme(&app, theme);
    context.view(&app).await
}

#[tauri::command]
pub async fn refresh_subscription(
    app: AppHandle,
    context: State<'_, DesktopContext>,
) -> Result<AppView, AppError> {
    context.core()?.service.refresh().await?;
    context.view(&app).await
}

#[tauri::command]
pub async fn set_snapshot_enabled(
    app: AppHandle,
    context: State<'_, DesktopContext>,
    enabled: bool,
) -> Result<AppView, AppError> {
    context.core()?.service.set_snapshot(enabled).await?;
    context.view(&app).await
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    context: State<'_, DesktopContext>,
) -> Result<AppView, AppError> {
    let core = context.core()?;
    core.server
        .start(core.service.status().await.settings.port)
        .await?;
    *context.startup_error.lock().await = None;
    context.view(&app).await
}

#[tauri::command]
pub async fn stop_server(
    app: AppHandle,
    context: State<'_, DesktopContext>,
) -> Result<AppView, AppError> {
    context.core()?.server.stop().await?;
    *context.startup_error.lock().await = None;
    context.view(&app).await
}

#[tauri::command]
pub async fn set_autostart_enabled(
    app: AppHandle,
    context: State<'_, DesktopContext>,
    enabled: bool,
) -> Result<AppView, AppError> {
    autostart::set_enabled(&autostart::SystemAutostart::new(app.clone()), enabled)?;
    context.view(&app).await
}

#[tauri::command]
pub fn quit_application(app: AppHandle) {
    desktop::request_exit(&app);
}
