pub mod cmd;
pub mod conversion;
pub mod model;
pub mod preset;
pub mod ssr;
pub mod subscription;

use cmd::{convert_ssr_to_clash, parse_ssr_link_command};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            convert_ssr_to_clash,
            parse_ssr_link_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
