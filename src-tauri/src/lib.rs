pub mod cmd;
pub mod model;
pub mod ssr;

use cmd::convert_ssr_to_clash;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![convert_ssr_to_clash])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
