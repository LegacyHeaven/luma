// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod logging;
mod shortcuts;
mod themes;
mod tray;
mod updater;
mod window;

use commands::AppState;
use std::sync::Mutex;
use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

fn main() {
    tauri::Builder::default()
        .manage(logging::AppLog::new())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch (double-clicking the binary again, or the OS
            // relaunching it) shouldn't spawn a second Luma - focus the
            // window from the instance that's already running instead.
            // Without this, two processes end up racing for the same
            // global shortcut and the loser's Alt+Space silently no-ops.
            //
            // If you're staring at the debug console wondering why a fix
            // "isn't taking effect": this line firing means you're looking
            // at an OLD process that never fully quit - check the build
            // sha in the system-info panel against what you just
            // downloaded. Quit Luma from the tray icon (not just closing
            // the window) and relaunch to be sure you're on the new build.
            logging::info(app, "second instance launch detected - focusing existing window instead of starting a new one");
            window::show_main_window(app);
        }))
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
                            cfg.window.spotlight_custom_x,
                            cfg.window.spotlight_custom_y,
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
            commands::open_position_picker,
            commands::report_spotlight_position,
            commands::cancel_position_pick,
            commands::reset_spotlight_position,
            commands::add_custom_engine,
            commands::remove_custom_engine,
            commands::search_mypc,
            logging::get_system_info,
            logging::get_debug_log,
            logging::clear_debug_log,
            logging::log_client_event,
            updater::check_for_update,
            updater::apply_update,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            logging::info(
                &handle,
                format!(
                    "LUMA {} (build {}) starting up, pid {}",
                    env!("CARGO_PKG_VERSION"),
                    logging::BUILD_SHA,
                    std::process::id()
                ),
            );

            let cfg = config::load(&handle);
            config::ensure_themes_dir(&handle);

            // The window array in tauri.conf.json creates the main window
            // at its default size before this closure ever runs, so a
            // saved "compact"/"roomy" preference needs to be applied here -
            // a no-op resize when it's already "default".
            window::apply_main_window_size(&handle, &cfg.window.main_window_size);

            app.manage(AppState {
                config: Mutex::new(cfg.clone()),
            });

            if let Err(err) = shortcuts::register(&handle, &cfg.general.shortcut) {
                logging::error(
                    &handle,
                    format!(
                        "could not register shortcut '{}': {err} - falling back to Alt+Space",
                        cfg.general.shortcut
                    ),
                );
                let fallback =
                    tauri_plugin_global_shortcut::Shortcut::new(Some(Modifiers::ALT), Code::Space);
                if let Err(err) = handle.global_shortcut().register(fallback) {
                    logging::error(&handle, format!("fallback Alt+Space registration also failed: {err}"));
                }
            } else {
                logging::info(&handle, format!("registered global shortcut '{}'", cfg.general.shortcut));
            }

            if cfg.general.start_at_login {
                #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                {
                    use tauri_plugin_autostart::ManagerExt;
                    let _ = handle.autolaunch().enable();
                }
            }

            tray::build(&handle)?;
            logging::info(&handle, "tray icon ready, setup complete");

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
