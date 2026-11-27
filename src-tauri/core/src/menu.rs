use crate::types::{CachedBuild, Channel, PublicState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchEntry {
    pub tag: String,
    pub label: String,
    pub active: bool,
}

fn build_number(version: &str) -> &str {
    version.rsplit('.').next().unwrap_or(version)
}

// Matches the names the window gives builds in src/builds.ts.
pub fn build_title(build: &CachedBuild) -> String {
    let tag = build.tag.as_str();
    if build.channel == Channel::Stable {
        return tag.to_string();
    }
    if tag.starts_with("nightly-") {
        return format!("Nightly #{}", build_number(&build.version));
    }
    if let Some(pr) = tag.strip_prefix("preview-pr-") {
        return format!("PR #{pr}");
    }
    if tag.starts_with("preview-manual-") {
        return format!("Manual build #{}", build_number(&build.version));
    }
    if let Some(branch) = tag.strip_prefix("preview-branch-") {
        return format!("Branch {branch}");
    }
    tag.to_string()
}

pub fn switch_entries(state: &PublicState) -> Vec<SwitchEntry> {
    let mut builds: Vec<&CachedBuild> = state.builds.iter().collect();
    builds.sort_by(|a, b| b.downloaded_at.cmp(&a.downloaded_at));
    builds
        .into_iter()
        .map(|build| SwitchEntry {
            tag: build.tag.clone(),
            label: build_title(build),
            active: state.active_tag.as_deref() == Some(build.tag.as_str()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached(tag: &str, version: &str, channel: Channel, downloaded_at: &str) -> CachedBuild {
        CachedBuild {
            tag: tag.to_string(),
            version: version.to_string(),
            channel,
            downloaded_at: downloaded_at.to_string(),
        }
    }

    fn state(builds: Vec<CachedBuild>, active: Option<&str>) -> PublicState {
        PublicState {
            active_tag: active.map(str::to_string),
            active_path: None,
            data_dir: String::new(),
            builds,
            train: None,
            pending_reload: None,
            last_train_check: None,
        }
    }

    #[test]
    fn names_builds_like_the_window() {
        let cases = [
            (cached("v1.6.0", "1.6.0", Channel::Stable, ""), "v1.6.0"),
            (
                cached("nightly-45599017613", "1.6.0.1886", Channel::Preview, ""),
                "Nightly #1886",
            ),
            (cached("preview-pr-670", "1.6.0.1890", Channel::Preview, ""), "PR #670"),
            (
                cached("preview-manual-123", "1.6.0.1891", Channel::Preview, ""),
                "Manual build #1891",
            ),
            (
                cached("preview-branch-redesign", "1.6.0.1892", Channel::Preview, ""),
                "Branch redesign",
            ),
            (cached("preview-odd", "1.6.0", Channel::Preview, ""), "preview-odd"),
        ];
        for (build, title) in cases {
            assert_eq!(build_title(&build), title);
        }
    }

    #[test]
    fn lists_newest_download_first_and_marks_the_active_one() {
        let entries = switch_entries(&state(
            vec![
                cached("v1.5.0", "1.5.0", Channel::Stable, "2026-11-01T00:00:00Z"),
                cached("nightly-2", "1.6.0.1886", Channel::Preview, "2026-11-20T00:00:00Z"),
                cached("nightly-1", "1.6.0.1873", Channel::Preview, "2026-11-10T00:00:00Z"),
            ],
            Some("nightly-1"),
        ));
        let summary: Vec<(&str, bool)> = entries
            .iter()
            .map(|e| (e.label.as_str(), e.active))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("Nightly #1886", false),
                ("Nightly #1873", true),
                ("v1.5.0", false),
            ]
        );
    }

    #[test]
    fn no_downloaded_builds_gives_no_entries() {
        assert!(switch_entries(&state(vec![], None)).is_empty());
    }
}
