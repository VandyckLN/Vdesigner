#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vdesigner::commands;
use vdesigner::picker::Picker;
use vdesigner::session::Session;
use vdesigner::wheel::Wheel;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Session::default())
        .manage(Picker::default())
        .manage(Wheel::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_image,
            commands::preview,
            commands::estimate,
            commands::preview_svg,
            commands::export,
            commands::load_palette,
            commands::save_palette,
            commands::color_variations,
            commands::format_color,
            commands::gradient_css,
            commands::start_pick,
            commands::pick_at,
            commands::cancel_pick,
            commands::start_wheel,
            commands::cancel_wheel,
            commands::convert_selection,
        ])
        .setup(|app| {
            use tauri::{Emitter, Manager};
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

            const ATALHO_COR: &str = "Ctrl+Alt+C";
            const ATALHO_RODA: &str = "Ctrl+Alt+V";
            let combinacao = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);
            let combinacao_roda =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);
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
                            // The button path shows its own error; the shortcut has
                            // no caller to return one to, so a failure here must be
                            // sent to the main window or the person just sees nothing.
                            if let Err(mensagem) =
                                vdesigner::commands::open_picker(&main_handle, &picker)
                            {
                                let _ = main_handle.emit_to(
                                    "main",
                                    "pick-failed",
                                    PickFailed { mensagem },
                                );
                            }
                        }
                    });
                })
                .is_ok();

            // The wheel reads the File Explorer selection, so unlike the
            // eyedropper it has nothing to fall back on: there is no button
            // inside the app that could stand in for "the files I have
            // highlighted out there". A dead shortcut here is the whole
            // feature gone, which is why it warns the same way.
            let registrado_roda = app
                .global_shortcut()
                .on_shortcut(combinacao_roda, move |app_handle, _shortcut, event| {
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    let main_handle = app_handle.clone();
                    let _ = app_handle.run_on_main_thread(move || {
                        if let Some(wheel) = main_handle.try_state::<vdesigner::wheel::Wheel>() {
                            // The shortcut has no caller to return an error
                            // to, so a failure has to reach the main window
                            // or the person just sees nothing happen.
                            if let Err(mensagem) =
                                vdesigner::commands::open_wheel(&main_handle, &wheel)
                            {
                                let _ = main_handle.emit_to(
                                    "main",
                                    "wheel-failed",
                                    WheelFailed { mensagem },
                                );
                            }
                        }
                    });
                })
                .is_ok();

            for (atalho, ok) in [(ATALHO_COR, registrado), (ATALHO_RODA, registrado_roda)] {
                if ok {
                    continue;
                }
                let payload = ShortcutUnavailable {
                    atalho: atalho.to_string(),
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
            if let tauri::WindowEvent::Destroyed = event {
                use tauri::Manager;
                if window.label() == vdesigner::picker::OVERLAY_LABEL {
                    if let Some(picker) = window.try_state::<vdesigner::picker::Picker>() {
                        picker.disarm();
                    }
                }
                // The selection is small, but leaving it armed would let a
                // later `convert_selection` run against files the person
                // already dismissed the wheel for.
                if window.label() == vdesigner::wheel::WHEEL_LABEL {
                    if let Some(wheel) = window.try_state::<vdesigner::wheel::Wheel>() {
                        wheel.disarm();
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

#[derive(Clone, serde::Serialize)]
struct PickFailed {
    mensagem: String,
}

#[derive(Clone, serde::Serialize)]
struct WheelFailed {
    mensagem: String,
}
