use crate::adapters;
use crate::model::{AgentInstance, AgentStatus, UpdateMode, UpdateResult};

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn publisher(&self) -> &'static str;
    fn official_url(&self) -> &'static str;
    fn discover(&self) -> Result<AgentInstance, String>;
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String>;
    fn update(&self, agent: &mut AgentInstance) -> Result<UpdateResult, String>;
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String>;
}

pub fn all_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(adapters::ClaudeAdapter),
        Box::new(adapters::CodexAdapter),
        Box::new(adapters::HermesAdapter),
        Box::new(adapters::OpenClawAdapter),
        Box::new(adapters::WorkBuddyAdapter),
        Box::new(adapters::MarvisAdapter),
    ]
}

pub fn adapter_for(id: &str) -> Option<Box<dyn AgentAdapter>> {
    all_adapters().into_iter().find(|adapter| adapter.id() == id)
}

pub fn not_installed(id: &str, name: &str, publisher: &str, url: &str, detail: &str) -> AgentInstance {
    AgentInstance {
        id: id.to_string(),
        name: name.to_string(),
        publisher: publisher.to_string(),
        installed: false,
        install_path: None,
        executable_path: None,
        version: None,
        latest_version: None,
        status: AgentStatus::NotInstalled,
        running: false,
        update_mode: if id == "workbuddy" || id == "marvis" { UpdateMode::ManualAction } else { UpdateMode::NativeUpdater },
        official_url: url.to_string(),
        detail: detail.to_string(),
        last_checked: None,
    }
}


