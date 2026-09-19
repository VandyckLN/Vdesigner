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
            commands::estimate,
            commands::export
        ])
        .run(tauri::generate_context!())
        // Tauri's own bootstrap idiom: no recovery is possible if the builder
        // itself fails to start, so this is the one carve-out from the
        // no-`unwrap`/no-`expect` rule the rest of the codebase follows.
        .expect("erro ao iniciar o Vdesigner");
}
