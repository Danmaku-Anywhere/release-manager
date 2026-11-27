use release_manager_core::manager::ReleaseManager;
use release_manager_core::menu::switch_entries;
use release_manager_core::types::PublicState;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

pub const HIDDEN_ARG: &str = "--hidden";

const TRAY_ID: &str = "main";
const SHOW: &str = "show";
const UPDATE: &str = "update";
const LOGIN: &str = "login";
const QUIT: &str = "quit";
const BUILDS: &str = "builds";
const SWITCH_PREFIX: &str = "switch:";

// Only exists once the tray icon is up. Without a tray there is no way back to
// a hidden window, so closing the window quits as it used to.
pub struct Tray {
    update: MenuItem<Wry>,
    pending_update: std::sync::Mutex<Option<String>>,
    builds: Submenu<Wry>,
}

pub fn has_tray(app: &AppHandle) -> bool {
    app.try_state::<Tray>().is_some()
}

pub fn show_main_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let name = &app.package_info().name;
    let show = MenuItem::with_id(app, SHOW, format!("Show {name}"), true, None::<&str>)?;
    let update = MenuItem::with_id(app, UPDATE, "Check for app updates", true, None::<&str>)?;
    let login_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let login = CheckMenuItem::with_id(
        app,
        LOGIN,
        "Open at login",
        true,
        login_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT, format!("Quit {name}"), true, None::<&str>)?;
    let builds = Submenu::with_id(app, BUILDS, "Switch build", true)?;

    let menu = Menu::with_items(app, &[&show])?;
    // The Windows build has no update bundle to install.
    if cfg!(not(windows)) {
        menu.append(&update)?;
    }
    menu.append_items(&[
        &PredefinedMenuItem::separator(app)?,
        &builds,
        &PredefinedMenuItem::separator(app)?,
        &login,
        &PredefinedMenuItem::separator(app)?,
        &quit,
    ])?;

    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(name)
        .menu(&menu)
        .on_menu_event(move |app, event| on_menu_event(app, &event, &login));

    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/tray-template.png"
        ))?)
        .icon_as_template(true);

    #[cfg(not(target_os = "macos"))]
    let builder = builder.icon(tauri::image::Image::from_bytes(include_bytes!(
        "../icons/tray-color.png"
    ))?);

    // Windows users expect a left click on a notification-area icon to open the
    // app and a right click to show its menu.
    #[cfg(windows)]
    let builder = builder
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    builder.build(app)?;
    app.manage(Tray {
        update,
        pending_update: std::sync::Mutex::new(None),
        builds,
    });

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<ReleaseManager>().get_state().await;
        refresh_builds(&handle, &state);
    });
    Ok(())
}

pub fn refresh_builds(app: &AppHandle, state: &PublicState) {
    let Some(tray) = app.try_state::<Tray>() else {
        return;
    };
    if let Err(err) = fill_builds(app, &tray.builds, state) {
        eprintln!("could not update the tray build list: {err}");
    }
}

fn fill_builds(app: &AppHandle, submenu: &Submenu<Wry>, state: &PublicState) -> tauri::Result<()> {
    for item in submenu.items()? {
        submenu.remove(&item)?;
    }
    let entries = switch_entries(state);
    if entries.is_empty() {
        submenu.append(&MenuItem::new(app, "No downloaded builds", false, None::<&str>)?)?;
    }
    for entry in entries {
        submenu.append(&CheckMenuItem::with_id(
            app,
            format!("{SWITCH_PREFIX}{}", entry.tag),
            entry.label,
            true,
            entry.active,
            None::<&str>,
        )?)?;
    }
    Ok(())
}

fn switch_to(app: &AppHandle, tag: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let manager = app.state::<ReleaseManager>();
        // Picking the active build again would copy it over itself and ask for
        // another Chrome reload, so it only puts the check mark back.
        if manager.get_state().await.active_tag.as_deref() != Some(tag.as_str()) {
            if let Err(err) = manager.set_active(&tag).await {
                crate::notifications::notify(
                    &app,
                    crate::notifications::Kind::SwitchFailed,
                    "Could not switch builds",
                    &err.to_string(),
                );
            }
        }
        let state = manager.get_state().await;
        refresh_builds(&app, &state);
        let _ = app.emit("state-changed", state);
    });
}

fn on_menu_event(app: &AppHandle, event: &MenuEvent, login: &CheckMenuItem<Wry>) {
    match event.id().as_ref() {
        SHOW => show_main_window(app),
        UPDATE => {
            let pending = app
                .try_state::<Tray>()
                .and_then(|tray| tray.pending_update.lock().ok()?.clone());
            if pending.is_some() {
                let _ = app.emit("tray-install-update", ());
            } else {
                show_main_window(app);
                let _ = app.emit("tray-check-update", ());
            }
        }
        LOGIN => {
            // The menu has already flipped the check mark by the time this runs.
            let want = login.is_checked().unwrap_or(false);
            let autolaunch = app.autolaunch();
            let result = if want {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };
            if result.is_err() {
                let _ = login.set_checked(!want);
            }
        }
        QUIT => app.exit(0),
        id => {
            if let Some(tag) = id.strip_prefix(SWITCH_PREFIX) {
                switch_to(app, tag.to_string());
            }
        }
    }
}

#[tauri::command]
pub fn set_tray_update(app: AppHandle, version: Option<String>) {
    let Some(tray) = app.try_state::<Tray>() else {
        return;
    };
    let name = &app.package_info().name;
    let (label, tooltip) = match &version {
        Some(version) => (
            format!("Update to {version} and restart"),
            format!("{name}: update {version} is ready"),
        ),
        None => ("Check for app updates".to_string(), name.clone()),
    };
    let _ = tray.update.set_text(label);
    if let Some(icon) = app.tray_by_id(TRAY_ID) {
        let _ = icon.set_tooltip(Some(tooltip));
    }
    let Ok(mut pending) = tray.pending_update.lock() else {
        return;
    };
    if let Some(found) = version.as_ref().filter(|v| pending.as_ref() != Some(*v)) {
        crate::notifications::notify(
            &app,
            crate::notifications::Kind::AppUpdate,
            &format!("{name} {found} is available"),
            if cfg!(target_os = "macos") {
                "Click to install it and restart."
            } else {
                "Choose Update from the tray icon to install it."
            },
        );
    }
    *pending = version;
}
