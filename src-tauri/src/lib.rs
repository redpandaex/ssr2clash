pub mod cmd;
pub mod conversion;
pub mod desktop;
pub mod model;
pub mod preset;
pub mod server;
pub mod service;
pub mod ssr;
pub mod storage;
pub mod subscription;

use desktop::{lifecycle, DesktopContext};
use std::sync::atomic::Ordering;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !lifecycle::start_hidden(args.iter().map(String::as_str)) {
                desktop::show_window(app);
            }
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("SSR2Clash")
                .args(["--autostart"])
                .build(),
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let context = tauri::async_runtime::block_on(DesktopContext::initialize(app.handle()));
            let initialized = context.core().is_ok();
            let restore_error = tauri::async_runtime::block_on(async {
                if let Ok(core) = context.core() {
                    if lifecycle::should_restore(
                        core.service.should_restore_service().await,
                        initialized,
                    ) {
                        return core
                            .server
                            .start(core.service.status().await.settings.port)
                            .await
                            .err()
                            .map(|error| error.message);
                    }
                }
                None
            });
            *context.startup_error.blocking_lock() = restore_error.clone();
            app.manage(context);
            desktop::tray::build(app.handle())?;
            if let Ok(core) = app.state::<DesktopContext>().core() {
                let status = tauri::async_runtime::block_on(core.service.status());
                desktop::apply_theme(app.handle(), status.settings.theme);
                desktop::tray::update(app.handle(), &status);
            }
            desktop::attach_state_events(app.handle());
            if !lifecycle::start_hidden(
                std::env::args()
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str),
            ) || !initialized
                || restore_error.is_some()
            {
                desktop::show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::ThemeChanged(_)) {
                desktop::tray::refresh_icon(window.app_handle());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    let app = window.app_handle();
                    let context = app.state::<DesktopContext>();
                    let close_to_tray = if let Ok(core) = context.core() {
                        core.service.status().await.settings.close_to_tray
                    } else {
                        false
                    };
                    match lifecycle::close_action(close_to_tray, false) {
                        lifecycle::CloseAction::Hide => {
                            let _ = window.hide();
                        }
                        lifecycle::CloseAction::Quit => desktop::request_exit(app),
                    }
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd::get_state,
            cmd::get_output_details,
            cmd::save_settings,
            cmd::refresh_subscription,
            cmd::set_snapshot_enabled,
            cmd::start_server,
            cmd::stop_server,
            cmd::set_autostart_enabled,
            cmd::quit_application,
        ])
        .build(tauri::generate_context!())
        .expect("无法初始化 SSR2Clash 桌面环境");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !app
                .state::<DesktopContext>()
                .quitting
                .load(Ordering::Acquire)
            {
                api.prevent_exit();
                desktop::request_exit(app);
            }
        }
    });
}
