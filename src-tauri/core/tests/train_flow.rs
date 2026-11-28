// Drives the auto update train against a fake in-process GitHub server whose
// release list can change between checks. Asserts which build ends up in active/
// on disk, the recorded check result, the pending Chrome reload, pruning, and that
// a failed or disabled check leaves active/ alone.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use release_manager_core::{
    active::active_path,
    cache::cache_dir,
    manager::ReleaseManager,
    types::{Train, TrainCheckStatus, TrainKind},
};
use tempfile::{tempdir, TempDir};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn make_zip(version: &str) -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("manifest.json", opts).unwrap();
    zip.write_all(format!(r#"{{"version":"{version}"}}"#).as_bytes())
        .unwrap();
    zip.finish().unwrap().into_inner()
}

#[derive(Clone)]
struct Release {
    tag: String,
    version: String,
    prerelease: bool,
    published_at: String,
}

fn release(tag: &str, version: &str, published_at: &str) -> Release {
    Release {
        tag: tag.to_string(),
        version: version.to_string(),
        prerelease: !tag.starts_with('v'),
        published_at: published_at.to_string(),
    }
}

#[derive(Default)]
struct Shared {
    pages: Mutex<Vec<Vec<Release>>>,
    failing: AtomicBool,
    release_requests: AtomicUsize,
    asset_requests: AtomicUsize,
}

struct FakeGithub {
    base: String,
    shared: Arc<Shared>,
    server: tokio::task::JoinHandle<()>,
}

impl FakeGithub {
    async fn start(pages: Vec<Vec<Release>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let shared = Arc::new(Shared::default());
        *shared.pages.lock().unwrap() = pages;

        let server = {
            let shared = Arc::clone(&shared);
            let base = base.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((mut stream, _)) = listener.accept().await else {
                        break;
                    };
                    let shared = Arc::clone(&shared);
                    let base = base.clone();
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 8192];
                        let n = stream.read(&mut buf).await.unwrap_or(0);
                        let head = String::from_utf8_lossy(&buf[..n]);
                        let path = head
                            .lines()
                            .next()
                            .and_then(|l| l.split_whitespace().nth(1))
                            .unwrap_or("")
                            .to_string();
                        let (status, body) = respond(&shared, &base, &path);
                        let header = format!(
                            "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(&body).await;
                    });
                }
            })
        };

        FakeGithub {
            base,
            shared,
            server,
        }
    }

    fn set_pages(&self, pages: Vec<Vec<Release>>) {
        *self.shared.pages.lock().unwrap() = pages;
    }

    fn set_failing(&self, failing: bool) {
        self.shared.failing.store(failing, Ordering::SeqCst);
    }

    fn release_requests(&self) -> usize {
        self.shared.release_requests.load(Ordering::SeqCst)
    }

    fn asset_requests(&self) -> usize {
        self.shared.asset_requests.load(Ordering::SeqCst)
    }

    fn manager(&self, data_dir: &Path) -> ReleaseManager {
        release_manager_core::ensure_crypto_provider();
        let client = reqwest::Client::builder()
            .user_agent("train-test")
            .build()
            .unwrap();
        ReleaseManager::with_client(
            data_dir.to_path_buf(),
            Some(format!("{}/releases", self.base)),
            client,
        )
    }
}

impl Drop for FakeGithub {
    fn drop(&mut self) {
        self.server.abort();
    }
}

fn page_of(path: &str) -> usize {
    path.split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("page="))
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
}

fn respond(shared: &Shared, base: &str, path: &str) -> (u16, Vec<u8>) {
    if shared.failing.load(Ordering::SeqCst) {
        return (500, b"down".to_vec());
    }
    let pages = shared.pages.lock().unwrap();
    if path.starts_with("/releases") {
        shared.release_requests.fetch_add(1, Ordering::SeqCst);
        let page = pages.get(page_of(path) - 1).cloned().unwrap_or_default();
        let entries: Vec<String> = page
            .iter()
            .map(|r| {
                format!(
                    r#"{{"tag_name":"{tag}","prerelease":{pre},"published_at":"{at}","assets":[{{"name":"danmaku-anywhere-{version}-chrome.zip","url":"{base}/assets/{tag}"}}]}}"#,
                    tag = r.tag,
                    pre = r.prerelease,
                    at = r.published_at,
                    version = r.version,
                )
            })
            .collect();
        return (200, format!("[{}]", entries.join(",")).into_bytes());
    }
    if let Some(tag) = path.strip_prefix("/assets/") {
        shared.asset_requests.fetch_add(1, Ordering::SeqCst);
        let found = pages.iter().flatten().find(|r| r.tag == tag);
        return match found {
            Some(r) => (200, make_zip(&r.version)),
            None => (404, b"not found".to_vec()),
        };
    }
    (404, b"not found".to_vec())
}

fn active_version(data_dir: &Path) -> String {
    let raw = std::fs::read_to_string(active_path(data_dir).join("manifest.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed["version"].as_str().unwrap().to_string()
}

fn nightly(keep_last: Option<u32>) -> Option<Train> {
    Some(Train {
        kind: TrainKind::Nightly,
        keep_last,
    })
}

fn two_nightlies_and_a_stable() -> Vec<Vec<Release>> {
    vec![vec![
        release("nightly-200", "1.5.0.20", "2026-11-20T00:00:00Z"),
        release("preview-pr-7", "1.5.0.25", "2026-11-20T12:00:00Z"),
        release("nightly-100", "1.5.0.10", "2026-11-19T00:00:00Z"),
        release("v1.5.0", "1.5.0", "2026-02-07T00:00:00Z"),
    ]]
}

async fn setup(pages: Vec<Vec<Release>>) -> (FakeGithub, TempDir, ReleaseManager) {
    let github = FakeGithub::start(pages).await;
    let dir = tempdir().unwrap();
    let manager = github.manager(dir.path());
    (github, dir, manager)
}

#[tokio::test]
async fn nightly_train_follows_new_nightlies() {
    let (github, dir, manager) = setup(two_nightlies_and_a_stable()).await;

    let state = manager.set_train(nightly(None)).await.unwrap();
    assert_eq!(state.train, nightly(None));

    let state = manager.check_train().await.expect("first check");
    assert_eq!(state.active_tag.as_deref(), Some("nightly-200"));
    assert_eq!(state.pending_reload.as_deref(), Some("nightly-200"));
    assert_eq!(active_version(dir.path()), "1.5.0.20");
    let check = state.last_train_check.expect("check recorded");
    assert_eq!(check.status, TrainCheckStatus::Updated);
    assert_eq!(check.tag.as_deref(), Some("nightly-200"));

    let downloads = github.asset_requests();
    let state = manager.check_train().await.expect("second check");
    assert_eq!(github.asset_requests(), downloads, "no download when up to date");
    assert_eq!(
        state.last_train_check.unwrap().status,
        TrainCheckStatus::UpToDate
    );

    let mut pages = two_nightlies_and_a_stable();
    pages[0].insert(0, release("nightly-300", "1.5.0.30", "2026-11-21T00:00:00Z"));
    github.set_pages(pages);

    let state = manager.check_train().await.expect("third check");
    assert_eq!(state.active_tag.as_deref(), Some("nightly-300"));
    assert_eq!(active_version(dir.path()), "1.5.0.30");
    assert!(
        cache_dir(dir.path(), "nightly-200").exists(),
        "superseded build stays cached without keep_last"
    );
}

#[tokio::test]
async fn stable_train_finds_a_release_past_the_first_page() {
    let nightlies: Vec<Release> = (0..100)
        .map(|i| release(&format!("nightly-{i}"), &format!("1.5.0.{i}"), "2026-11-01T00:00:00Z"))
        .collect();
    let stables = vec![
        release("v1.5.0", "1.5.0", "2026-02-07T00:00:00Z"),
        release("v1.4.0", "1.4.0", "2026-01-07T00:00:00Z"),
    ];
    let (_github, dir, manager) = setup(vec![nightlies, stables]).await;

    manager
        .set_train(Some(Train {
            kind: TrainKind::Stable,
            keep_last: None,
        }))
        .await
        .unwrap();
    let state = manager.check_train().await.expect("check");

    assert_eq!(state.active_tag.as_deref(), Some("v1.5.0"));
    assert_eq!(active_version(dir.path()), "1.5.0");
}

#[tokio::test]
async fn failed_check_leaves_the_active_build_alone() {
    let (github, dir, manager) = setup(two_nightlies_and_a_stable()).await;
    manager.set_train(nightly(None)).await.unwrap();
    manager.check_train().await.expect("first check");

    github.set_failing(true);
    manager.check_train().await.expect_err("check must fail");

    let state = manager.get_state().await;
    assert_eq!(state.active_tag.as_deref(), Some("nightly-200"));
    assert_eq!(active_version(dir.path()), "1.5.0.20");
    let check = state.last_train_check.expect("failure recorded");
    assert_eq!(check.status, TrainCheckStatus::Failed);
    assert!(check.message.is_some());
}

#[tokio::test]
async fn picking_a_build_by_hand_turns_the_train_off() {
    let (github, dir, manager) = setup(two_nightlies_and_a_stable()).await;
    manager.set_train(nightly(None)).await.unwrap();
    manager.check_train().await.expect("check");

    manager.download_build("nightly-100").await.unwrap();
    let state = manager.set_active("nightly-100").await.unwrap();
    assert!(state.train.is_none());
    assert_eq!(state.pending_reload.as_deref(), Some("nightly-100"));

    let requests = github.release_requests();
    let state = manager.check_train().await.expect("check with train off");
    assert_eq!(github.release_requests(), requests, "no network when off");
    assert_eq!(state.active_tag.as_deref(), Some("nightly-100"));
    assert_eq!(active_version(dir.path()), "1.5.0.10");
}

#[tokio::test]
async fn keep_last_prunes_older_builds_on_the_train_only() {
    let (github, dir, manager) = setup(two_nightlies_and_a_stable()).await;
    manager.download_build("nightly-100").await.unwrap();
    manager.download_build("v1.5.0").await.unwrap();
    manager.set_train(nightly(Some(1))).await.unwrap();
    manager.check_train().await.expect("first check");

    let mut pages = two_nightlies_and_a_stable();
    pages[0].insert(0, release("nightly-300", "1.5.0.30", "2026-11-21T00:00:00Z"));
    github.set_pages(pages);
    let state = manager.check_train().await.expect("second check");

    let tags: Vec<&str> = state.builds.iter().map(|b| b.tag.as_str()).collect();
    assert!(tags.contains(&"nightly-300"));
    assert!(tags.contains(&"v1.5.0"), "other trains are not pruned");
    assert!(!tags.contains(&"nightly-200"));
    assert!(!tags.contains(&"nightly-100"));
    assert!(!cache_dir(dir.path(), "nightly-200").exists());
    assert!(!cache_dir(dir.path(), "nightly-100").exists());
}

#[tokio::test]
async fn no_build_on_the_train_is_reported_without_touching_active() {
    let (_github, _dir, manager) = setup(vec![vec![release(
        "preview-pr-7",
        "1.5.0.25",
        "2026-11-20T00:00:00Z",
    )]])
    .await;
    manager.set_train(nightly(None)).await.unwrap();

    let state = manager.check_train().await.expect("check");
    assert!(state.active_tag.is_none());
    assert_eq!(
        state.last_train_check.unwrap().status,
        TrainCheckStatus::NotFound
    );
}

#[tokio::test]
async fn train_and_pending_reload_survive_a_restart_until_dismissed() {
    let (github, dir, manager) = setup(two_nightlies_and_a_stable()).await;
    manager.set_train(nightly(Some(2))).await.unwrap();
    manager.check_train().await.expect("check");
    drop(manager);

    let manager = github.manager(dir.path());
    let state = manager.get_state().await;
    assert_eq!(state.train, nightly(Some(2)));
    assert_eq!(state.pending_reload.as_deref(), Some("nightly-200"));

    let state = manager.dismiss_reload().await.unwrap();
    assert!(state.pending_reload.is_none());
    assert_eq!(state.train, nightly(Some(2)));
}

#[tokio::test]
async fn stable_train_prefers_a_higher_version_over_a_later_backport() {
    let mut first: Vec<Release> = (0..99)
        .map(|i| {
            release(
                &format!("nightly-{i}"),
                &format!("1.5.0.{i}"),
                "2026-11-01T00:00:00Z",
            )
        })
        .collect();
    first.push(release("v1.4.2", "1.4.2", "2026-10-01T00:00:00Z"));
    let second = vec![release("v1.5.0", "1.5.0", "2026-02-07T00:00:00Z")];
    let (_github, dir, manager) = setup(vec![first, second]).await;

    manager
        .set_train(Some(Train {
            kind: TrainKind::Stable,
            keep_last: None,
        }))
        .await
        .unwrap();
    let state = manager.check_train().await.expect("check");

    assert_eq!(state.active_tag.as_deref(), Some("v1.5.0"));
    assert_eq!(active_version(dir.path()), "1.5.0");
}
