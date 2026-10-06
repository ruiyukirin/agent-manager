// Author: Kirin
use agent_manager::model::AgentInstance;
use std::{fs, path::PathBuf};

fn base_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\AppData\\Roaming"))
        .join("AgentManager")
}

pub fn state_path() -> PathBuf { base_dir().join("state.json") }
pub fn schedule_path() -> PathBuf { base_dir().join("schedule.json") }

pub fn save_agents(agents: &[AgentInstance]) -> Result<(), String> {
    let path = state_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(agents).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

pub fn save_schedule(config: &agent_manager::model::ScheduleConfig) -> Result<(), String> {
    let path = schedule_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

pub fn load_schedule() -> Result<agent_manager::model::ScheduleConfig, String> {
    let path = schedule_path();
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str(&text) { return Ok(config); }
    }
    Ok(agent_manager::model::ScheduleConfig { enabled: true, time: "02:00".into(), agent_ids: vec!["codex".into(), "hermes".into(), "openclaw".into(), "workbuddy".into(), "deepseek".into()] })
}

