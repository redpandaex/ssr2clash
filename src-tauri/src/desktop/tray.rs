use super::{request_exit, show_window, DesktopContext};
use crate::model::app::CoreStatus;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

struct TrayMenu {
    start: MenuItem<tauri::Wry>,
    stop: MenuItem<tauri::Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开 SSR2Clash", true, None::<&str>)?;
    let initialized = app.state::<DesktopContext>().core().is_ok();
    let start = MenuItem::with_id(app, "start", "启动本地服务", initialized, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "停止本地服务", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &start, &stop, &separator, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("SSR2Clash · 订阅转换与本地托管")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                show_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "quit" => request_exit(app),
            action @ ("start" | "stop") => {
                let start = action == "start";
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let context = app.state::<DesktopContext>();
                    if let Ok(core) = context.core() {
                        let result = if start {
                            core.server
                                .start(core.service.status().await.settings.port)
                                .await
                        } else {
                            core.server.stop().await
                        };
                        if let Err(error) = result {
                            *context.startup_error.lock().await = Some(error.message);
                            show_window(&app);
                        } else {
                            *context.startup_error.lock().await = None;
                        }
                        let _ = tauri::Emitter::emit(&app, "app-state-changed", ());
                    }
                });
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayMenu { start, stop });
    Ok(())
}

pub fn update(app: &AppHandle, status: &CoreStatus) {
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu
            .start
            .set_enabled(!status.server_running && !status.settings.profile.source_url.is_empty());
        let _ = menu.stop.set_enabled(status.server_running);
    }
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(if status.server_running {
            "SSR2Clash · 本地服务运行中"
        } else {
            "SSR2Clash · 本地服务已停止"
        }));
    }
}
