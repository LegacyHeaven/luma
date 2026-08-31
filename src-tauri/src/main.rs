// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod shortcuts;
mod themes;
mod tray;
mod window;

use commands::AppState;
use std::sync::Mutex;
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // Any registered shortcut currently just toggles the
                    // spotlight - Luma only ever registers the one hotkey
                    // from config.general.shortcut at a time.
                    if event.state() == ShortcutState::Pressed {
                        let state = app.state::<AppState>();
                        let cfg = state.config.lock().unwrap().clone();
                        window::toggle_spotlight(
                            app,
                            cfg.window.spotlight_width as f64,
                            &cfg.window.spotlight_position,
                        );
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::list_themes,
            commands::reveal_themes_folder,
            commands::get_engines,
            commands::get_theme_css,
            commands::open_result,
            commands::open_in_system_browser,
            commands::close_builtin_browser,
            commands::toggle_spotlight,
            commands::hide_spotlight,
            commands::show_main_window,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let cfg = config::load(&handle);
            config::ensure_themes_dir(&handle);

            app.manage(AppState {
                config: Mutex::new(cfg.clone()),
            });

            if let Err(err) = shortcuts::register(&handle, &cfg.general.shortcut) {
                eprintln!(
                    "luma: could not register shortcut '{}': {err} - falling back to Alt+Space",
                    cfg.general.shortcut
                );
                let fallback =
                    tauri_plugin_global_shortcut::Shortcut::new(Some(Modifiers::ALT), Code::Space);
                let _ = handle.global_shortcut().register(fallback);
            }

            if cfg.general.start_at_login {
                #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                {
                    use tauri_plugin_autostart::ManagerExt;
                    let _ = handle.autolaunch().enable();
                }
            }

            tray::build(&handle)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides it instead of quitting - Luma
            // keeps living in the tray so the global shortcut keeps working.
            // The spotlight window hides itself the same way on blur (see
            // the frontend's blur handler) rather than through this hook.
            if window.label() == window::MAIN_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running the Luma application");
}
