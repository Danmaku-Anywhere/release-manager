use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::active::{clear_active, recover_active, set_active};
use crate::cache::{download_build, reconcile_builds, remove_build};
use crate::error::RmError;
use crate::github::fetch_releases;
use crate::store::ConfigStore;
use crate::train::{builds_to_prune, newest_release};
use crate::types::{Config, PublicState, ReleaseAsset, Train, TrainCheck, TrainCheckStatus};

const MAX_RELEASE_PAGES: u32 = 10;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/Danmaku-Anywhere/danmaku-anywhere/releases";

pub(crate) fn validate_tag(tag: &str) -> Result<(), RmError> {
    if tag.is_empty()
        || tag == "."
        || tag == ".."
        || tag.contains('/')
        || tag.contains('\\')
        || tag.contains('\0')
        || std::path::Path::new(tag).file_name() != Some(OsStr::new(tag))
    {
        return Err(RmError::Invalid {
            message: format!("unsafe tag: {tag}"),
        });
    }
    Ok(())
}

pub struct ReleaseManager {
    data_dir: PathBuf,
    store: ConfigStore,
    github_base: String,
    client: reqwest::Client,
    in_flight: Arc<Mutex<HashSet<String>>>,
    config_lock: tokio::sync::Mutex<()>,
    train_lock: tokio::sync::Mutex<()>,
    last_train_check: Mutex<Option<TrainCheck>>,
}

// RAII guard: removes the tag from in_flight on drop, even if the future is cancelled.
// std::sync::Mutex is used because in_flight is only held for brief synchronous sections,
// never across an await, so a sync lock is both correct and reliably cleans up on drop.
struct InFlightGuard {
    in_flight: Arc<Mutex<HashSet<String>>>,
    tag: String,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.in_flight.lock().unwrap().remove(&self.tag);
    }
}

impl ReleaseManager {
    pub fn new(data_dir: PathBuf, github_base: Option<String>) -> Self {
        crate::ensure_crypto_provider();
        Self::with_client(data_dir, github_base, reqwest::Client::new())
    }

    pub fn with_client(
        data_dir: PathBuf,
        github_base: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        let store = ConfigStore::new(&data_dir);
        let github_base = github_base.unwrap_or_else(|| GITHUB_RELEASES_URL.to_string());
        ReleaseManager {
            data_dir,
            store,
            github_base,
            client,
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            config_lock: tokio::sync::Mutex::new(()),
            train_lock: tokio::sync::Mutex::new(()),
            last_train_check: Mutex::new(None),
        }
    }

    pub async fn get_state(&self) -> PublicState {
        let config = {
            let _lock = self.config_lock.lock().await;
            let config = self.store.load().await;
            self.reconcile_config(config).await
        };
        let mut state = self.store.to_public_state(&config, None);
        state.last_train_check = self.last_train_check.lock().unwrap().clone();
        state
    }

    // Called with config_lock held.
    async fn reconcile_config(&self, config: Config) -> Config {
        recover_active(&self.data_dir).await;
        let builds = match reconcile_builds(&self.data_dir, config.builds.clone()).await {
            Ok(b) => b,
            Err(_) => return config,
        };
        let active_still_present = config
            .active_tag
            .as_deref()
            .map(|t| builds.iter().any(|b| b.tag == t))
            .unwrap_or(false);

        if builds.len() == config.builds.len()
            && (config.active_tag.is_none() || active_still_present)
        {
            return config;
        }

        if config.active_tag.is_some() && !active_still_present {
            let _ = clear_active(&self.data_dir).await;
        }

        let reconciled = Config {
            builds,
            active_tag: if active_still_present {
                config.active_tag.clone()
            } else {
                None
            },
            ..config
        };
        let _ = self.store.save(&reconciled).await;
        reconciled
    }

    pub async fn list_releases(&self, page: u32) -> Result<Vec<ReleaseAsset>, RmError> {
        fetch_releases(&self.client, &self.github_base, None, page)
            .await
            .map(|(assets, _)| assets)
    }

    pub async fn download_build(&self, tag: &str) -> Result<PublicState, RmError> {
        validate_tag(tag)?;
        let _guard = self.claim_download(tag)?;
        let asset = self.find_release(tag).await?;
        self.install(&asset).await?;
        Ok(self.get_state().await)
    }

    fn claim_download(&self, tag: &str) -> Result<InFlightGuard, RmError> {
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if in_flight.contains(tag) {
                return Err(RmError::Conflict {
                    message: format!("a download for {tag} is already running"),
                });
            }
            in_flight.insert(tag.to_string());
        }

        Ok(InFlightGuard {
            in_flight: Arc::clone(&self.in_flight),
            tag: tag.to_string(),
        })
    }

    async fn search_releases<F>(&self, mut pick: F) -> Result<Option<ReleaseAsset>, RmError>
    where
        F: FnMut(&[ReleaseAsset]) -> Option<ReleaseAsset>,
    {
        let mut page = 1u32;
        loop {
            let (releases, raw_count) =
                fetch_releases(&self.client, &self.github_base, None, page).await?;
            if let Some(found) = pick(&releases) {
                return Ok(Some(found));
            }
            if raw_count < 100 || page >= MAX_RELEASE_PAGES {
                return Ok(None);
            }
            page += 1;
        }
    }

    async fn find_release(&self, tag: &str) -> Result<ReleaseAsset, RmError> {
        self.search_releases(|releases| releases.iter().find(|r| r.tag == tag).cloned())
            .await?
            .ok_or_else(|| RmError::NotFound {
                message: format!("no release tagged {tag}"),
            })
    }

    async fn install(&self, asset: &ReleaseAsset) -> Result<(), RmError> {
        let tag = asset.tag.as_str();
        let downloaded = download_build(&self.data_dir, asset, &self.client, None).await?;

        // Record the build and, if it is the active one, refresh active/ under a
        // single config_lock hold so the active tag cannot change between the
        // decision and the swap.
        let _lock = self.config_lock.lock().await;
        let mut config = self.store.load().await;
        config.builds.retain(|b| b.tag != tag);
        config.builds.push(downloaded);
        self.store.save(&config).await?;
        if config.active_tag.as_deref() == Some(tag) {
            set_active(&self.data_dir, tag).await?;
        }
        Ok(())
    }

    pub async fn set_active(&self, tag: &str) -> Result<PublicState, RmError> {
        validate_tag(tag)?;

        {
            let _lock = self.config_lock.lock().await;
            let config = self.store.load().await;
            if !config.builds.iter().any(|b| b.tag == tag) {
                return Err(RmError::NotFound {
                    message: format!("{tag} is not cached"),
                });
            }

            set_active(&self.data_dir, tag).await?;

            let mut updated = config;
            if updated.active_tag.as_deref() != Some(tag) {
                updated.train = None;
            }
            updated.active_tag = Some(tag.to_string());
            updated.pending_reload = Some(tag.to_string());
            self.store.save(&updated).await?;
        }

        Ok(self.get_state().await)
    }

    pub async fn set_train(&self, train: Option<Train>) -> Result<PublicState, RmError> {
        {
            let _lock = self.config_lock.lock().await;
            let mut config = self.store.load().await;
            config.train = train;
            self.store.save(&config).await?;
        }
        *self.last_train_check.lock().unwrap() = None;
        Ok(self.get_state().await)
    }

    pub async fn dismiss_reload(&self) -> Result<PublicState, RmError> {
        {
            let _lock = self.config_lock.lock().await;
            let mut config = self.store.load().await;
            config.pending_reload = None;
            self.store.save(&config).await?;
        }
        Ok(self.get_state().await)
    }

    pub async fn check_train(&self) -> Result<PublicState, RmError> {
        let _running = self.train_lock.lock().await;
        let Some(train) = self.store.load().await.train else {
            return Ok(self.get_state().await);
        };

        let outcome = self.follow_train(&train).await;
        if self.store.load().await.train.as_ref() != Some(&train) {
            return Ok(self.get_state().await);
        }
        let checked_at = chrono::Utc::now().to_rfc3339();
        let check = match &outcome {
            Ok(Some((tag, status))) => TrainCheck {
                checked_at,
                status: status.clone(),
                tag: Some(tag.clone()),
                message: None,
            },
            Ok(None) => TrainCheck {
                checked_at,
                status: TrainCheckStatus::NotFound,
                tag: None,
                message: None,
            },
            Err(e) => TrainCheck {
                checked_at,
                status: TrainCheckStatus::Failed,
                tag: None,
                message: Some(e.to_string()),
            },
        };
        *self.last_train_check.lock().unwrap() = Some(check);
        outcome?;
        Ok(self.get_state().await)
    }

    async fn follow_train(
        &self,
        train: &Train,
    ) -> Result<Option<(String, TrainCheckStatus)>, RmError> {
        // A backport can be published after a higher version, so the newest
        // release on the train may sit on any page.
        let mut on_train = Vec::new();
        self.search_releases(|releases| {
            on_train.extend(newest_release(train.kind, releases).cloned());
            None
        })
        .await?;
        let newest = newest_release(train.kind, &on_train).cloned();
        let Some(newest) = newest else {
            return Ok(None);
        };
        let tag = newest.tag.clone();
        validate_tag(&tag)?;

        let cached = self.store.load().await.builds.iter().any(|b| b.tag == tag);
        if !cached {
            let _guard = self.claim_download(&tag)?;
            self.install(&newest).await?;
        }
        let status = self.activate_for_train(train, &tag).await?;
        Ok(status.map(|status| (tag, status)))
    }

    // The train can be changed or turned off while its build downloads, so the
    // swap and the pruning both check it again under config_lock. When it changed,
    // nothing is touched and this returns None.
    async fn activate_for_train(
        &self,
        train: &Train,
        tag: &str,
    ) -> Result<Option<TrainCheckStatus>, RmError> {
        let _lock = self.config_lock.lock().await;
        let mut config = self.store.load().await;
        if config.train.as_ref() != Some(train) {
            return Ok(None);
        }
        let status = if config.active_tag.as_deref() == Some(tag) {
            TrainCheckStatus::UpToDate
        } else {
            set_active(&self.data_dir, tag).await?;
            config.active_tag = Some(tag.to_string());
            config.pending_reload = Some(tag.to_string());
            self.store.save(&config).await?;
            TrainCheckStatus::Updated
        };
        if let Some(keep_last) = train.keep_last {
            let doomed = builds_to_prune(
                train.kind,
                keep_last,
                &config.builds,
                config.active_tag.as_deref(),
            );
            for tag in &doomed {
                validate_tag(tag)?;
                remove_build(&self.data_dir, tag).await?;
            }
            if !doomed.is_empty() {
                config.builds.retain(|b| !doomed.contains(&b.tag));
                self.store.save(&config).await?;
            }
        }
        Ok(Some(status))
    }

    pub async fn remove_build(&self, tag: &str) -> Result<PublicState, RmError> {
        validate_tag(tag)?;

        {
            let _lock = self.config_lock.lock().await;
            let config = self.store.load().await;
            if config.active_tag.as_deref() == Some(tag) {
                return Err(RmError::Conflict {
                    message: format!(
                        "{tag} is active; set another build active before removing it"
                    ),
                });
            }

            remove_build(&self.data_dir, tag).await?;

            let mut updated = config;
            updated.builds.retain(|b| b.tag != tag);
            self.store.save(&updated).await?;
        }

        Ok(self.get_state().await)
    }

    pub async fn reconcile(&self) {
        let _lock = self.config_lock.lock().await;
        let config = self.store.load().await;
        self.reconcile_config(config).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::cache_dir;
    use tempfile::tempdir;

    #[tokio::test]
    async fn remove_build_refuses_active_tag() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        let store = crate::store::ConfigStore::new(&data_dir);
        let tag = "v1.0.0";
        let cache = cache_dir(&data_dir, tag);
        tokio::fs::create_dir_all(&cache).await.unwrap();
        tokio::fs::write(cache.join("manifest.json"), r#"{"version":"1.0.0"}"#)
            .await
            .unwrap();
        let config = crate::types::Config {
            active_tag: Some(tag.to_string()),
            builds: vec![crate::types::CachedBuild {
                tag: tag.to_string(),
                version: "1.0.0".to_string(),
                channel: crate::types::Channel::Stable,
                downloaded_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            ..Default::default()
        };
        store.save(&config).await.unwrap();

        let manager = ReleaseManager::new(data_dir, Some("http://localhost".to_string()));
        let result = manager.remove_build(tag).await;
        assert!(result.is_err());
        if let Err(RmError::Conflict { .. }) = result {
        } else {
            panic!("expected Conflict error");
        }
    }

    #[tokio::test]
    async fn reconcile_drops_missing_dirs() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        let store = crate::store::ConfigStore::new(&data_dir);
        let config = crate::types::Config {
            active_tag: None,
            builds: vec![crate::types::CachedBuild {
                tag: "ghost".to_string(),
                version: "1.0.0".to_string(),
                channel: crate::types::Channel::Preview,
                downloaded_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            ..Default::default()
        };
        store.save(&config).await.unwrap();

        let manager = ReleaseManager::new(data_dir, Some("http://localhost".to_string()));
        manager.reconcile().await;

        let reloaded = store.load().await;
        assert!(reloaded.builds.is_empty());
    }

    #[tokio::test]
    async fn reconcile_clears_dangling_active() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path().to_path_buf();
        let store = crate::store::ConfigStore::new(&data_dir);

        // The tag is recorded as active but has no cache directory.
        let config = crate::types::Config {
            active_tag: Some("ghost".to_string()),
            builds: vec![crate::types::CachedBuild {
                tag: "ghost".to_string(),
                version: "1.0.0".to_string(),
                channel: crate::types::Channel::Stable,
                downloaded_at: "2026-01-01T00:00:00Z".to_string(),
            }],
            ..Default::default()
        };
        store.save(&config).await.unwrap();

        let manager = ReleaseManager::new(data_dir, Some("http://localhost".to_string()));
        manager.reconcile().await;

        let reloaded = store.load().await;
        assert!(reloaded.active_tag.is_none());
        assert!(reloaded.builds.is_empty());
    }

    #[test]
    fn validate_tag_accepts_valid_tags() {
        assert!(validate_tag("v1.2.0").is_ok());
        assert!(validate_tag("nightly-abc").is_ok());
        assert!(validate_tag("preview-pr-1").is_ok());
    }

    #[test]
    fn validate_tag_rejects_traversal() {
        assert!(validate_tag("../../evil").is_err());
        assert!(validate_tag("").is_err());
        assert!(validate_tag(".").is_err());
        assert!(validate_tag("..").is_err());
        assert!(validate_tag("a/b").is_err());
        assert!(validate_tag("a\\b").is_err());
    }
}
