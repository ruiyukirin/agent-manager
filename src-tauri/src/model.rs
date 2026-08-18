use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSnapshot {
    pub product: Option<String>,
    pub component: Option<String>,
    pub bootstrap: Option<String>,
    pub channel: Option<String>,
    pub pe: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AgentStatus {
    NotInstalled,
    Checking,
    UpToDate,
    UpdateAvailable,
    ManualAction,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateMode {
    NativeUpdater,
    StagedInstaller,
    ManualAction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallMethod {
    Winget { #[serde(rename = "packageId")] package_id: String },
    DirectDownload { url: String },
    OpenBrowser { url: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstance {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub installed: bool,
    pub install_path: Option<String>,
    pub executable_path: Option<String>,
    pub version: Option<VersionSnapshot>,
    pub latest_version: Option<String>,
    pub status: AgentStatus,
    pub running: bool,
    pub update_mode: UpdateMode,
    pub official_url: String,
    pub install_url: String,
    pub install_method: InstallMethod,
    pub detail: String,
    pub last_checked: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleConfig {
    pub enabled: bool,
    pub time: String,
    pub agent_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub mode: UpdateMode,
    pub official_url: Option<String>,
    pub needs_restart: bool,
    pub previous_version: Option<String>,
    pub current_version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub method: InstallMethod,
    pub needs_restart: bool,
}
