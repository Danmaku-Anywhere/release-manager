use std::path::PathBuf;
use std::sync::Mutex;

use release_manager_core::types::{PublicState, TrainCheckStatus};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationSettings {
    pub app_updates: bool,
    pub new_builds: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            app_updates: true,
            new_builds: true,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Kind {
    AppUpdate,
    AppUpdated,
    NewBuild,
    SwitchFailed,
}

#[cfg(target_os = "macos")]
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Kind::AppUpdate => "app-update",
            Kind::AppUpdated => "app-updated",
            Kind::NewBuild => "new-build",
            Kind::SwitchFailed => "switch-failed",
        }
    }
}

// Clicking a notification does what it is about: install the app update, or
// open Chrome's extensions page to reload the new build.
#[cfg(target_os = "macos")]
pub fn on_click(app: &AppHandle, kind: &str) {
    use tauri::Emitter;
    if kind == Kind::AppUpdate.id() {
        let _ = app.emit("tray-install-update", ());
    } else if kind == Kind::NewBuild.id() {
        let _ = crate::commands::open_extensions_page();
    } else {
        crate::tray::show_main_window(app);
    }
}

pub struct Notifications {
    file: Option<PathBuf>,
    settings: Mutex<NotificationSettings>,
}

impl Notifications {
    pub fn load(app: &AppHandle) -> Self {
        let file = app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.join("notifications.json"));
        let settings = file
            .as_ref()
            .and_then(|file| std::fs::read(file).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            file,
            settings: Mutex::new(settings),
        }
    }

    fn get(&self) -> NotificationSettings {
        self.settings.lock().map(|s| *s).unwrap_or_default()
    }

    fn set(&self, settings: NotificationSettings) -> Result<(), String> {
        if let Ok(mut current) = self.settings.lock() {
            *current = settings;
        }
        let Some(file) = &self.file else {
            return Ok(());
        };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        std::fs::write(file, json).map_err(|e| e.to_string())
    }
}

pub fn notify(app: &AppHandle, kind: Kind, title: &str, body: &str) {
    let settings = app
        .try_state::<Notifications>()
        .map(|n| n.get())
        .unwrap_or_default();
    let wanted = match kind {
        Kind::AppUpdate | Kind::AppUpdated => settings.app_updates,
        Kind::NewBuild => settings.new_builds,
        Kind::SwitchFailed => true,
    };
    if wanted {
        show(app, kind, title, body);
    }
}

#[cfg(target_os = "macos")]
fn show(_app: &AppHandle, kind: Kind, title: &str, body: &str) {
    crate::notify_mac::show(kind.id(), title, body);
}

#[cfg(not(target_os = "macos"))]
fn show(app: &AppHandle, _kind: Kind, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

// Called after each background train check. A check only reports Updated when
// it moved a new build into the folder.
pub fn announce_train_update(app: &AppHandle, state: &PublicState) {
    let Some(check) = &state.last_train_check else {
        return;
    };
    if !matches!(check.status, TrainCheckStatus::Updated) {
        return;
    }
    let Some(tag) = &check.tag else {
        return;
    };
    let version = state
        .builds
        .iter()
        .find(|build| &build.tag == tag)
        .map_or(tag.as_str(), |build| build.version.as_str());
    notify(
        app,
        Kind::NewBuild,
        &format!("Danmaku Anywhere {version} is ready"),
        if cfg!(target_os = "macos") {
            "Click to open Chrome extensions and reload it."
        } else {
            "Reload the extension in Chrome to start using it."
        },
    );
}

// Remembers the last version that ran, so the first start after a self-update
// can say it happened.
pub fn announce_new_version(app: &AppHandle) {
    let Ok(dir) = app.path().app_data_dir() else {
        return;
    };
    let marker = dir.join("last-run-version");
    let current = app.package_info().version.to_string();
    let previous = std::fs::read_to_string(&marker).ok();
    if previous
        .as_deref()
        .is_some_and(|prev| prev.trim() != current)
    {
        let name = &app.package_info().name;
        notify(
            app,
            Kind::AppUpdated,
            &format!("{name} updated to {current}"),
            "You're on the new version.",
        );
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&marker, &current);
}

#[tauri::command]
pub fn get_notification_settings(app: AppHandle) -> NotificationSettings {
    app.try_state::<Notifications>()
        .map(|n| n.get())
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_notification_settings(
    app: AppHandle,
    settings: NotificationSettings,
) -> Result<NotificationSettings, String> {
    if let Some(notifications) = app.try_state::<Notifications>() {
        notifications.set(settings)?;
    }
    Ok(settings)
}
