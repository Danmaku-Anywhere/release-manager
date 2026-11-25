use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    pub tag: String,
    pub version: String,
    pub channel: Channel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_subtype: Option<PreviewSubtype>,
    pub published_at: String,
    pub asset_url: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedBuild {
    pub tag: String,
    pub version: String,
    pub channel: Channel,
    pub downloaded_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tag: Option<String>,
    pub builds: Vec<CachedBuild>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub train: Option<Train>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_reload: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrainKind {
    Stable,
    Nightly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Train {
    pub kind: TrainKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_last: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrainCheckStatus {
    UpToDate,
    Updated,
    NotFound,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainCheck {
    pub checked_at: String,
    pub status: TrainCheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_path: Option<String>,
    pub data_dir: String,
    pub builds: Vec<CachedBuild>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train: Option<Train>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_reload: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_train_check: Option<TrainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    Stable,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewSubtype {
    Nightly,
    Pr,
    Branch,
    Manual,
    Generic,
}
