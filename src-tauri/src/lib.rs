//! Ekstre desktop shell: Tauri setup, menu-bar tray, and close-to-tray window
//! behavior. Business logic lives in `ekstre_core`; this wires it to the UI.

mod commands;
mod scheduler;
mod state;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir).ok();
            let resource_dir = app.path().resource_dir().ok();
            app.manage(AppState::new(data_dir, resource_dir));

            let show = MenuItem::with_id(app, "show", "Panoyu aç", true, None::<&str>)?;
            let poll = MenuItem::with_id(app, "poll", "Şimdi tara", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Ayarlar", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &poll, &settings, &quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Ekstre")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                            let _ = w.eval("window.location.href = 'settings.html'");
                        }
                    }
                    "poll" => {
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            let state = handle.state::<AppState>();
                            match state.run_poll() {
                                Ok(n) => log::info!("poll added {n} statements"),
                                Err(e) => log::warn!("poll failed: {e}"),
                            }
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Launch at login in release builds (skipped in dev to avoid adding
            // the dev binary to login items).
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }

            scheduler::start(app.handle().clone());

            check_for_updates(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app alive in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_statements,
            commands::get_calendar,
            commands::is_configured,
            commands::list_banks,
            commands::poll_now,
            commands::test_imap,
            commands::complete_setup,
            commands::update_settings,
            commands::get_settings,
            commands::save_settings,
            commands::download_statement,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ekstre");
}

/// Check for an available update and, if found, download + install it, then
/// relaunch. Runs in the background so it never blocks startup.
fn check_for_updates(app: tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                log::warn!("updater unavailable: {e}");
                return;
            }
        };
        match updater.check().await {
            Ok(Some(update)) => {
                log::info!("update {} available; installing", update.version);
                if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
                    log::warn!("update install failed: {e}");
                } else {
                    app.restart();
                }
            }
            Ok(None) => log::info!("no update available"),
            Err(e) => log::warn!("update check failed: {e}"),
        }
    });
}
