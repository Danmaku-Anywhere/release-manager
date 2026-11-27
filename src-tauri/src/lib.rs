use std::path::PathBuf;
use std::time::Duration;

use release_manager_core::manager::ReleaseManager;
use tauri::{AppHandle, Emitter, Manager};

mod commands;
mod notifications;
#[cfg(target_os = "macos")]
mod notify_mac;
mod tray;

const TRAIN_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

// The first tick fires at once, so this also covers the startup check.
async fn follow_train(app: AppHandle) {
    let mut ticker = tokio::time::interval(TRAIN_CHECK_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let manager = app.state::<ReleaseManager>();
        if let Ok(state) = manager.check_train().await {
            notifications::announce_train_update(&app, &state);
        }
        let state = manager.get_state().await;
        tray::refresh_builds(&app, &state);
        let _ = app.emit("state-changed", state);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_single_instance::init(
        |app, _args, _cwd| {
            tray::show_main_window(app);
        },
    ));
    #[cfg(not(target_os = "macos"))]
    let builder = builder.plugin(tauri_plugin_notification::init());
    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![tray::HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_dir = std::env::var("DA_RELEASE_MANAGER_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    app.path()
                        .home_dir()
                        .expect("cannot resolve home directory")
                        .join(".da-release-manager")
                });

            let github_base = std::env::var("DA_RELEASE_MANAGER_GITHUB_BASE").ok();

            // GitHub returns 403 for requests without a User-Agent header.
            let client = reqwest::Client::builder()
                .user_agent("danmaku-anywhere-release-manager")
                .connect_timeout(Duration::from_secs(15))
                .read_timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client");

            let manager = ReleaseManager::with_client(data_dir, github_base, client);
            app.manage(manager);
            app.manage(notifications::Notifications::load(app.handle()));
            tauri::async_runtime::spawn(follow_train(app.handle().clone()));

            // The bundle icon only covers the installed app. Set it on the window
            // too so the title bar and switcher show the real icon in dev and on
            // X11 desktops. GNOME Wayland ignores window icons and matches the
            // installed .desktop instead, so dev there still shows a generic icon.
            if let Some(window) = app.get_webview_window("main") {
                let info = app.package_info();
                let _ = window.set_title(&format!("{} {}", info.name, info.version));
                let icon = tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/128x128.png"
                ))?;
                let _ = window.set_icon(icon);
            }

            let handle = app.handle();
            #[cfg(target_os = "macos")]
            {
                let clicked = handle.clone();
                notify_mac::init(move |kind| notifications::on_click(&clicked, kind));
            }
            if let Err(err) = tray::create(handle) {
                eprintln!("tray icon unavailable, closing the window will quit: {err}");
            }
            let start_hidden = std::env::args().any(|arg| arg == tray::HIDDEN_ARG);
            if start_hidden && tray::has_tray(handle) {
                tray::hide_main_window(handle);
            } else {
                tray::show_main_window(handle);
            }
            notifications::announce_new_version(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && tray::has_tray(window.app_handle()) {
                    api.prevent_close();
                    tray::hide_main_window(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::list_releases,
            commands::download_build,
            commands::set_active,
            commands::remove_build,
            commands::set_train,
            commands::check_train,
            commands::dismiss_reload,
            commands::open_extensions_page,
            tray::set_tray_update,
            notifications::get_notification_settings,
            notifications::set_notification_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Opening the app again from Finder, Spotlight or the Dock while it is
    // hidden in the menu bar.
    #[cfg(target_os = "macos")]
    app.run(|handle, event| {
        if let tauri::RunEvent::Reopen { .. } = event {
            tray::show_main_window(handle);
        }
    });
    #[cfg(not(target_os = "macos"))]
    app.run(|_, _| {});
}
