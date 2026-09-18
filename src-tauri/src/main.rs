#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vdesigner::commands;
use vdesigner::session::Session;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Session::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_image,
            commands::preview,
            commands::export
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Vdesigner");
}
