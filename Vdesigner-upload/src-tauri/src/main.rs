#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vdesigner::commands;
use vdesigner::picker::Picker;
use vdesigner::session::Session;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Session::default())
        .manage(Picker::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_image,
            commands::preview,
            commands::estimate,
            commands::export,
            commands::load_palette,
            commands::save_palette,
            commands::color_variations,
            commands::format_color,
            commands::gradient_css,
            commands::start_pick,
            commands::pick_at,
            commands::cancel_pick,
        ])
        .setup(|app| {
            use tauri::{Emitter, Manager};
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

            const ATALHO: &str = "Ctrl+Alt+C";
            let combinacao = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);
            let handle = app.handle().clone();

            // Registration fails silently by nature when another program
            // already owns the combination. A dead shortcut the person
            // believes in is worse than no shortcut, so say so.
            let registrado = app
                .global_shortcut()
                .on_shortcut(combinacao, move |app_handle, _shortcut, event| {
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    let main_handle = app_handle.clone();
                    let _ = app_handle.run_on_main_thread(move || {
                        if let Some(existing) =
                            main_handle.get_webview_window(vdesigner::picker::OVERLAY_LABEL)
                        {
                            let _ = existing.hide();
                            let _ = existing.close();
                        }
                        if let Some(picker) = main_handle.try_state::<vdesigner::picker::Picker>() {
                            picker.disarm();
                            let _ = vdesigner::commands::start_pick(main_handle.clone(), picker);
                        }
                    });
                })
                .is_ok();

            if !registrado {
                let payload = ShortcutUnavailable {
                    atalho: ATALHO.to_string(),
                };
                let _ = handle.emit_to("main", "shortcut-unavailable", payload.clone());

                let handle_task = handle.clone();
                let payload_task = payload.clone();
                std::thread::spawn(move || {
                    for delay_ms in [200, 600, 1200] {
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        let _ = handle_task.emit_to(
                            "main",
                            "shortcut-unavailable",
                            payload_task.clone(),
                        );
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == vdesigner::picker::OVERLAY_LABEL {
                if let tauri::WindowEvent::Destroyed = event {
                    use tauri::Manager;
                    if let Some(picker) = window.try_state::<vdesigner::picker::Picker>() {
                        picker.disarm();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        // Tauri's own bootstrap idiom: no recovery is possible if the builder
        // itself fails to start, so this is the one carve-out from the
        // no-`unwrap`/no-`expect` rule the rest of the codebase follows.
        .expect("erro ao iniciar o Vdesigner");
}

#[derive(Clone, serde::Serialize)]
struct ShortcutUnavailable {
    atalho: String,
}
