use std::path::{Path, PathBuf};

use tokio::fs;

use crate::active::active_path;
use crate::error::RmError;
use crate::types::{CachedBuild, Config, PublicState};

const CONFIG_FILE: &str = "config.json";

pub struct ConfigStore {
    data_dir: PathBuf,
    config_path: PathBuf,
}

impl ConfigStore {
    pub fn new(data_dir: &Path) -> Self {
        let config_path = data_dir.join(CONFIG_FILE);
        ConfigStore {
            data_dir: data_dir.to_path_buf(),
            config_path,
        }
    }

    pub async fn load(&self) -> Config {
        let raw = match fs::read_to_string(&self.config_path).await {
            Ok(s) => s,
            Err(_) => return Config::default(),
        };
        match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => {
                let active_tag = v["activeTag"].as_str().map(|s| s.to_string());
                let builds: Vec<CachedBuild> = v["builds"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|b| serde_json::from_value(b.clone()).ok())
                            .collect()
                    })
                    .unwrap_or_default();
                let train = serde_json::from_value(v["train"].clone()).ok();
                let pending_reload = v["pendingReload"].as_str().map(|s| s.to_string());
                Config {
                    active_tag,
                    builds,
                    train,
                    pending_reload,
                }
            }
            Err(_) => Config::default(),
        }
    }

    pub async fn save(&self, config: &Config) -> Result<(), RmError> {
        fs::create_dir_all(&self.data_dir)
            .await
            .map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
        let json = serde_json::to_string_pretty(config).map_err(|e| RmError::Invalid {
            message: e.to_string(),
        })?;
        let tmp_path = self.config_path.with_extension("json.tmp");
        #[cfg(unix)]
        {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_path)
                .await
                .map_err(|e| RmError::Swap {
                    message: e.to_string(),
                })?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|e| RmError::Swap {
                    message: e.to_string(),
                })?;
            file.flush().await.map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
            file.sync_all().await.map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&tmp_path, &json)
                .await
                .map_err(|e| RmError::Swap {
                    message: e.to_string(),
                })?;
        }
        fs::rename(&tmp_path, &self.config_path)
            .await
            .map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
        Ok(())
    }

    pub fn to_public_state(&self, config: &Config, active_path_override: Option<String>) -> PublicState {
        let active_path = if config.active_tag.is_some() {
            active_path_override.or_else(|| {
                Some(active_path(&self.data_dir).to_string_lossy().into_owned())
            })
        } else {
            None
        };
        PublicState {
            active_tag: config.active_tag.clone(),
            active_path,
            data_dir: self.data_dir.to_string_lossy().into_owned(),
            builds: config.builds.clone(),
            train: config.train.clone(),
            pending_reload: config.pending_reload.clone(),
            last_train_check: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn returns_default_when_no_config() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        let config = store.load().await;
        assert!(config.builds.is_empty());
        assert!(config.active_tag.is_none());
    }

    #[tokio::test]
    async fn round_trips_config() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        let config = Config {
            active_tag: Some("v1.0.0".to_string()),
            builds: vec![crate::types::CachedBuild {
                tag: "v1.0.0".to_string(),
                version: "1.0.0".to_string(),
                channel: crate::types::Channel::Stable,
                downloaded_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            ..Config::default()
        };
        store.save(&config).await.unwrap();

        let reloaded = ConfigStore::new(dir.path()).load().await;
        assert_eq!(reloaded.active_tag, Some("v1.0.0".to_string()));
        assert_eq!(reloaded.builds.len(), 1);
    }

    #[tokio::test]
    async fn round_trips_train_and_pending_reload() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        let config = Config {
            train: Some(crate::types::Train {
                kind: crate::types::TrainKind::Nightly,
                keep_last: Some(3),
            }),
            pending_reload: Some("nightly-1".to_string()),
            ..Config::default()
        };
        store.save(&config).await.unwrap();

        let reloaded = ConfigStore::new(dir.path()).load().await;
        assert_eq!(reloaded, config);
    }

    #[tokio::test]
    async fn config_without_train_loads_with_train_off() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"activeTag":"v1.0.0","builds":[]}"#,
        )
        .unwrap();
        let config = ConfigStore::new(dir.path()).load().await;
        assert_eq!(config.active_tag.as_deref(), Some("v1.0.0"));
        assert!(config.train.is_none());
        assert!(config.pending_reload.is_none());
    }

    #[tokio::test]
    async fn unknown_train_kind_loads_as_off() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"builds":[],"train":{"kind":"weekly"}}"#,
        )
        .unwrap();
        let config = ConfigStore::new(dir.path()).load().await;
        assert!(config.train.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn file_permissions_are_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        store.save(&Config::default()).await.unwrap();

        let meta = std::fs::metadata(dir.path().join("config.json")).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }

    #[tokio::test]
    async fn to_public_state_has_no_token_field() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path());
        let config = Config::default();
        let state = store.to_public_state(&config, None);
        let serialized = serde_json::to_string(&state).unwrap();
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("hasToken"));
    }
}
