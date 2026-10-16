use crate::{
    conversion::compile_subscription,
    model::{app::ProfileConfig, convert::ConvertConfig},
    preset::load_builtin_preset,
    ssr::parser::parse_ssr_link,
};
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
    context.core()?.service.save_settings(settings).await?;
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

#[tauri::command]
pub fn convert_ssr_to_clash(
    ssr_links: Vec<String>,
    config: ConvertConfig,
) -> Result<String, String> {
    let profile = ProfileConfig {
        filter_keywords: config.filter_keywords,
        exclude_keywords: config.exclude_keywords,
        enable_udp: config.enable_udp,
        ..ProfileConfig::default()
    };
    let preset = load_builtin_preset().map_err(|error| error.to_string())?;
    compile_subscription(&ssr_links.join("\n"), &profile, &preset, 0)
        .map(|result| result.output.yaml)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn parse_ssr_link_command(link: String) -> Result<String, String> {
    let node = parse_ssr_link(&link).map_err(|error| format!("解析失败: {error}"))?;
    serde_json::to_string(&node).map_err(|_| "序列化节点失败".to_owned())
}
