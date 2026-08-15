use agent_manager::model::AgentInstance;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub agents: Mutex<Vec<AgentInstance>>,
}

