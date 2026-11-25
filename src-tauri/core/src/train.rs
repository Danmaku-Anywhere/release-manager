use std::cmp::Ordering;

use crate::types::{CachedBuild, Channel, ReleaseAsset, TrainKind};

pub fn on_train(kind: TrainKind, tag: &str, channel: &Channel) -> bool {
    match kind {
        TrainKind::Stable => *channel == Channel::Stable && tag.starts_with('v'),
        TrainKind::Nightly => *channel == Channel::Preview && tag.starts_with("nightly-"),
    }
}

// Nightly versions are `<base>.<run number>`, and the run number only goes up,
// so comparing the dotted numbers in order works for both trains.
fn version_key(version: &str) -> Option<Vec<u64>> {
    version
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().ok()
        })
        .collect()
}

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    match (version_key(a), version_key(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

pub fn newest_release(kind: TrainKind, releases: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    releases
        .iter()
        .filter(|r| on_train(kind, &r.tag, &r.channel))
        .max_by(|a, b| {
            compare_versions(&a.version, &b.version).then_with(|| a.published_at.cmp(&b.published_at))
        })
}

pub fn builds_to_prune(
    kind: TrainKind,
    keep_last: u32,
    builds: &[CachedBuild],
    active_tag: Option<&str>,
) -> Vec<String> {
    let mut on: Vec<&CachedBuild> = builds
        .iter()
        .filter(|b| on_train(kind, &b.tag, &b.channel))
        .collect();
    on.sort_by(|a, b| compare_versions(&b.version, &a.version));
    on.into_iter()
        .skip(keep_last as usize)
        .filter(|b| Some(b.tag.as_str()) != active_tag)
        .map(|b| b.tag.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, version: &str, channel: Channel, published_at: &str) -> ReleaseAsset {
        ReleaseAsset {
            tag: tag.to_string(),
            version: version.to_string(),
            channel,
            preview_subtype: None,
            published_at: published_at.to_string(),
            asset_url: String::new(),
            notes: vec![],
        }
    }

    fn cached(tag: &str, version: &str, channel: Channel) -> CachedBuild {
        CachedBuild {
            tag: tag.to_string(),
            version: version.to_string(),
            channel,
            downloaded_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn stable_train_takes_only_v_tags_that_are_not_prereleases() {
        assert!(on_train(TrainKind::Stable, "v1.5.0", &Channel::Stable));
        assert!(!on_train(TrainKind::Stable, "v1.6.0-beta", &Channel::Preview));
        assert!(!on_train(TrainKind::Stable, "release-manager-v0.2.0", &Channel::Stable));
        assert!(!on_train(TrainKind::Stable, "nightly-1", &Channel::Preview));
    }

    #[test]
    fn nightly_train_takes_only_nightly_prereleases() {
        assert!(on_train(TrainKind::Nightly, "nightly-44888381130", &Channel::Preview));
        assert!(!on_train(TrainKind::Nightly, "preview-pr-618", &Channel::Preview));
        assert!(!on_train(TrainKind::Nightly, "v1.5.0", &Channel::Stable));
    }

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.5.0.1873", "1.5.0.999"), Ordering::Greater);
        assert_eq!(compare_versions("1.6.0.10", "1.5.0.1873"), Ordering::Greater);
        assert_eq!(compare_versions("1.5.0", "1.5.0"), Ordering::Equal);
    }

    #[test]
    fn newest_nightly_is_the_highest_run_number_not_the_newest_tag() {
        let releases = vec![
            release("nightly-900", "1.5.0.1873", Channel::Preview, "2026-11-21T00:00:00Z"),
            release("nightly-950", "1.5.0.1862", Channel::Preview, "2026-11-22T00:00:00Z"),
            release("preview-pr-1", "1.5.0.1900", Channel::Preview, "2026-11-23T00:00:00Z"),
            release("v1.5.0", "1.5.0", Channel::Stable, "2026-02-07T00:00:00Z"),
        ];
        let newest = newest_release(TrainKind::Nightly, &releases).unwrap();
        assert_eq!(newest.tag, "nightly-900");
    }

    #[test]
    fn newest_stable_is_the_highest_semver() {
        let releases = vec![
            release("v1.4.1", "1.4.1", Channel::Stable, "2026-03-01T00:00:00Z"),
            release("v1.5.0", "1.5.0", Channel::Stable, "2026-02-07T00:00:00Z"),
            release("nightly-1", "1.5.0.1873", Channel::Preview, "2026-11-21T00:00:00Z"),
        ];
        let newest = newest_release(TrainKind::Stable, &releases).unwrap();
        assert_eq!(newest.tag, "v1.5.0");
    }

    #[test]
    fn no_release_on_train_gives_none() {
        let releases = vec![release("preview-pr-1", "1.5.0.1", Channel::Preview, "")];
        assert!(newest_release(TrainKind::Stable, &releases).is_none());
    }

    #[test]
    fn prunes_beyond_keep_last_and_spares_the_active_build() {
        let builds = vec![
            cached("nightly-a", "1.5.0.10", Channel::Preview),
            cached("nightly-b", "1.5.0.30", Channel::Preview),
            cached("nightly-c", "1.5.0.20", Channel::Preview),
            cached("nightly-d", "1.5.0.5", Channel::Preview),
            cached("preview-pr-1", "1.5.0.1", Channel::Preview),
            cached("v1.5.0", "1.5.0", Channel::Stable),
        ];
        let pruned = builds_to_prune(TrainKind::Nightly, 2, &builds, Some("nightly-d"));
        assert_eq!(pruned, vec!["nightly-a".to_string()]);
    }
}
