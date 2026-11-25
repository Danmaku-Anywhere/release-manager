use std::path::{Path, PathBuf};

use tokio::fs;

use crate::cache::cache_dir;
use crate::error::RmError;

const ACTIVE_DIR: &str = "active";
const NEXT_DIR: &str = "active.next";
const PREV_DIR: &str = "active.prev";

fn swap_error(e: std::io::Error) -> RmError {
    RmError::Swap {
        message: e.to_string(),
    }
}

async fn remove_any(path: &Path) -> Result<(), RmError> {
    let meta = match fs::symlink_metadata(path).await {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(swap_error(e)),
    };
    if meta.file_type().is_dir() {
        fs::remove_dir_all(path).await.map_err(swap_error)
    } else {
        remove_non_dir(path, meta.file_type())
            .await
            .map_err(swap_error)
    }
}

pub fn active_path(data_dir: &Path) -> PathBuf {
    data_dir.join(ACTIVE_DIR)
}

// Windows only deletes a directory symlink or junction through remove_dir;
// remove_file on one fails with access denied.
#[cfg(windows)]
async fn remove_non_dir(path: &Path, file_type: std::fs::FileType) -> std::io::Result<()> {
    use std::os::windows::fs::FileTypeExt;
    if file_type.is_symlink_dir() {
        fs::remove_dir(path).await
    } else {
        fs::remove_file(path).await
    }
}

#[cfg(not(windows))]
async fn remove_non_dir(path: &Path, _file_type: std::fs::FileType) -> std::io::Result<()> {
    fs::remove_file(path).await
}

async fn ensure_real_dir(dir: &Path) -> Result<(), RmError> {
    if let Ok(meta) = fs::symlink_metadata(dir).await {
        if !meta.file_type().is_dir() {
            remove_non_dir(dir, meta.file_type())
                .await
                .map_err(|e| RmError::Swap {
                    message: e.to_string(),
                })?;
        }
    }
    fs::create_dir_all(dir).await.map_err(|e| RmError::Swap {
        message: e.to_string(),
    })
}

async fn empty_dir(dir: &Path) -> Result<(), RmError> {
    let mut entries = match fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(RmError::Swap {
                message: e.to_string(),
            })
        }
    };
    loop {
        let entry = entries.next_entry().await.map_err(|e| RmError::Swap {
            message: e.to_string(),
        })?;
        let Some(entry) = entry else {
            break;
        };
        let path = entry.path();
        let is_dir = entry
            .file_type()
            .await
            .map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?
            .is_dir();
        if is_dir {
            fs::remove_dir_all(&path).await.map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
        } else {
            fs::remove_file(&path).await.map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?;
        }
    }
    Ok(())
}

async fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), RmError> {
    fs::create_dir_all(dst).await.map_err(|e| RmError::Swap {
        message: e.to_string(),
    })?;
    let mut entries = fs::read_dir(src).await.map_err(|e| RmError::Swap {
        message: e.to_string(),
    })?;
    loop {
        let entry = entries.next_entry().await.map_err(|e| RmError::Swap {
            message: e.to_string(),
        })?;
        let Some(entry) = entry else {
            break;
        };
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let is_dir = entry
            .file_type()
            .await
            .map_err(|e| RmError::Swap {
                message: e.to_string(),
            })?
            .is_dir();
        if is_dir {
            Box::pin(copy_dir_all(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path)
                .await
                .map_err(|e| RmError::Swap {
                    message: e.to_string(),
                })?;
        }
    }
    Ok(())
}

// The active folder must be a real directory with copies of the build files, not a symlink.
// On Linux with Flatpak, the document portal resolves paths at access time and cannot
// follow symlinks outside the sandbox boundary.
//
// The new build is copied next to active/ first and then renamed into place, so a
// failed copy leaves the current build untouched. If the app dies between the two
// renames, active/ is missing and active.prev/ still holds the old build;
// recover_active puts it back.
pub async fn set_active(data_dir: &Path, tag: &str) -> Result<String, RmError> {
    let target = cache_dir(data_dir, tag);
    let dir = active_path(data_dir);
    let next = data_dir.join(NEXT_DIR);
    let prev = data_dir.join(PREV_DIR);

    if !tokio::fs::try_exists(&target).await.unwrap_or(false) {
        return Err(RmError::Swap {
            message: format!("cache dir for {tag} is missing"),
        });
    }

    remove_any(&next).await?;
    if let Err(e) = copy_dir_all(&target, &next).await {
        let _ = remove_any(&next).await;
        return Err(e);
    }

    remove_any(&prev).await?;
    match fs::symlink_metadata(&dir).await {
        Ok(meta) if meta.file_type().is_dir() => {
            fs::rename(&dir, &prev).await.map_err(swap_error)?;
        }
        Ok(meta) => {
            remove_non_dir(&dir, meta.file_type())
                .await
                .map_err(swap_error)?;
        }
        Err(_) => {}
    }
    if let Err(e) = fs::rename(&next, &dir).await {
        let _ = fs::rename(&prev, &dir).await;
        return Err(swap_error(e));
    }
    let _ = remove_any(&prev).await;

    Ok(dir.to_string_lossy().into_owned())
}

pub async fn recover_active(data_dir: &Path) {
    let dir = active_path(data_dir);
    let prev = data_dir.join(PREV_DIR);
    if fs::symlink_metadata(&dir).await.is_err() && fs::symlink_metadata(&prev).await.is_ok() {
        let _ = fs::rename(&prev, &dir).await;
    }
    let _ = remove_any(&data_dir.join(NEXT_DIR)).await;
}

pub async fn clear_active(data_dir: &Path) -> Result<(), RmError> {
    let dir = active_path(data_dir);
    ensure_real_dir(&dir).await?;
    empty_dir(&dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn setup_cache(data_dir: &Path, tag: &str, content: &str) {
        let cache = data_dir.join("cache").join(tag);
        fs::create_dir_all(&cache).await.unwrap();
        fs::write(cache.join("manifest.json"), content)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn set_active_copies_into_real_dir() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        setup_cache(data_dir, "v1", "one").await;

        let result = set_active(data_dir, "v1").await;
        assert!(result.is_ok());

        let active = active_path(data_dir);
        let meta = fs::symlink_metadata(&active).await.unwrap();
        assert!(meta.file_type().is_dir());
        assert!(!meta.file_type().is_symlink());

        let content = fs::read_to_string(active.join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(content, "one");
    }

    // Creating a real symlink on Windows needs Developer Mode or admin rights,
    // so the test uses a junction there, which any user can create. Both are
    // reparse points and both must be replaced by a real directory.
    #[cfg(unix)]
    fn link_dir(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(windows)]
    fn link_dir(target: &Path, link: &Path) {
        junction::create(target, link).unwrap();
    }

    #[tokio::test]
    async fn set_active_replaces_symlink_with_real_dir() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        setup_cache(data_dir, "v1", "one").await;
        setup_cache(data_dir, "v2", "two").await;

        let active = active_path(data_dir);
        let cache_v1 = data_dir.join("cache").join("v1");
        link_dir(&cache_v1, &active);
        let before = fs::symlink_metadata(&active).await.unwrap();
        assert!(before.file_type().is_symlink());

        let result = set_active(data_dir, "v2").await;
        assert!(result.is_ok(), "set_active failed: {:?}", result.err());

        let meta = fs::symlink_metadata(&active).await.unwrap();
        assert!(!meta.file_type().is_symlink());
        assert!(meta.is_dir());

        let content = fs::read_to_string(active.join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(content, "two");

        let untouched = fs::read_to_string(cache_v1.join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(
            untouched, "one",
            "removing the link must not touch its target"
        );
    }

    #[tokio::test]
    async fn set_active_fails_when_cache_missing() {
        let dir = tempdir().unwrap();
        let result = set_active(dir.path(), "does-not-exist").await;
        assert!(result.is_err());
        if let Err(RmError::Swap { .. }) = result {
        } else {
            panic!("expected Swap error");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_copy_keeps_the_current_build() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        setup_cache(data_dir, "v1", "one").await;
        setup_cache(data_dir, "v2", "two").await;
        set_active(data_dir, "v1").await.unwrap();

        let unreadable = data_dir.join("cache").join("v2").join("locked.js");
        fs::write(&unreadable, "x").await.unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = set_active(data_dir, "v2").await;
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(result.is_err());

        let active = active_path(data_dir);
        let content = fs::read_to_string(active.join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(content, "one");
        assert!(!data_dir.join(NEXT_DIR).exists());
    }

    #[tokio::test]
    async fn recover_active_restores_a_swap_cut_short() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        let prev = data_dir.join(PREV_DIR);
        fs::create_dir_all(&prev).await.unwrap();
        fs::write(prev.join("manifest.json"), "one").await.unwrap();

        recover_active(data_dir).await;

        let content = fs::read_to_string(active_path(data_dir).join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(content, "one");
        assert!(!prev.exists());
    }

    #[tokio::test]
    async fn clear_active_unlinks_a_link_without_touching_its_target() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        setup_cache(data_dir, "v1", "one").await;
        let cache_v1 = data_dir.join("cache").join("v1");
        let active = active_path(data_dir);
        link_dir(&cache_v1, &active);

        clear_active(data_dir).await.unwrap();

        let meta = fs::symlink_metadata(&active).await.unwrap();
        assert!(meta.is_dir());
        assert!(!meta.file_type().is_symlink());
        let untouched = fs::read_to_string(cache_v1.join("manifest.json"))
            .await
            .unwrap();
        assert_eq!(untouched, "one");
    }

    #[tokio::test]
    async fn clear_active_empties_the_dir() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        setup_cache(data_dir, "v1", "one").await;
        set_active(data_dir, "v1").await.unwrap();

        clear_active(data_dir).await.unwrap();

        let active = active_path(data_dir);
        let mut entries = fs::read_dir(&active).await.unwrap();
        assert!(entries.next_entry().await.unwrap().is_none());
    }
}
