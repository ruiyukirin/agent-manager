mod backup;
mod state;
mod storage;

use agent_manager::adapter::{adapter_for, all_adapters};
use agent_manager::model::{AgentInstance, ScheduleConfig, UpdateResult};
use state::AppState;
use std::{process::Command, sync::Mutex};

#[tauri::command]
fn discover_agents(state: tauri::State<AppState>) -> Result<Vec<AgentInstance>, String> {
    let mut agents = Vec::new();
    for adapter in all_adapters() {
        match adapter.discover() {
            Ok(agent) => agents.push(agent),
            Err(error) => eprintln!("{} discovery failed: {}", adapter.id(), error),
        }
    }
    storage::save_agents(&agents)?;
    *state.agents.lock().map_err(|e| e.to_string())? = agents.clone();
    Ok(agents)
}

#[tauri::command]
fn check_updates(agent_id: String, state: tauri::State<AppState>) -> Result<AgentInstance, String> {
    let mut agents = state.agents.lock().map_err(|e| e.to_string())?;
    let agent = agents.iter_mut().find(|agent| agent.id == agent_id).ok_or_else(|| format!("未找到 Agent: {}", agent_id))?;
    let adapter = adapter_for(&agent_id).ok_or_else(|| format!("没有适配器: {}", agent_id))?;
    adapter.check_latest(agent)?;
    let result = agent.clone();
    storage::save_agents(&agents)?;
    Ok(result)
}

#[tauri::command]
fn update_agent(agent_id: String, state: tauri::State<AppState>) -> Result<UpdateResult, String> {
    let mut agents = state.agents.lock().map_err(|e| e.to_string())?;
    let agent = agents.iter_mut().find(|agent| agent.id == agent_id).ok_or_else(|| format!("未找到 Agent: {}", agent_id))?;
    if !agent.installed {
        return Ok(UpdateResult {
            success: false,
            message: "该 Agent 尚未安装".into(),
            mode: agent.update_mode.clone(),
            official_url: Some(agent.official_url.clone()),
            needs_restart: false,
            previous_version: None,
            current_version: None,
        });
    }
    let previous = agent.version.as_ref().and_then(|version| version.product.clone());
    let was_running = agent.running;
    let _ = backup::backup_before_update(agent);
    let adapter = adapter_for(&agent_id).ok_or_else(|| format!("没有适配器: {}", agent_id))?;
    let mut result = adapter.update(agent)?;
    result.previous_version = previous;
    if result.success && was_running {
        adapter.restart_if_was_running(agent, was_running)?;
    }
    Ok(result)
}

#[tauri::command]
fn open_official_url(agent_id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let agents = state.agents.lock().map_err(|e| e.to_string())?;
    let url = agents
        .iter()
        .find(|agent| agent.id == agent_id)
        .map(|agent| agent.official_url.clone())
        .or_else(|| agent_for::url(&agent_id))
        .ok_or_else(|| format!("未找到 Agent: {}", agent_id))?;
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn()
        .map_err(|e| format!("无法打开官方入口: {}", e))?;
    Ok(())
}

mod agent_for {
    use super::adapter_for;
    pub fn url(id: &str) -> Option<String> {
        adapter_for(id).map(|adapter| adapter.official_url().to_string())
    }
}

#[tauri::command]
fn get_schedule(_state: tauri::State<AppState>) -> Result<ScheduleConfig, String> {
    storage::load_schedule()
}

#[tauri::command]
fn set_schedule(config: ScheduleConfig) -> Result<(), String> {
    if !config.enabled {
        let output = Command::new("schtasks.exe").args(["/Delete", "/TN", "Agent manager - Daily Check", "/F"]).output().map_err(|e| e.to_string())?;
        if !output.status.success() && !String::from_utf8_lossy(&output.stderr).contains("ERROR: The task does not exist") { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
        storage::save_schedule(&config)?;
        return Ok(());
    }
    storage::save_schedule(&config)?;
    let task = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    let action = format!("\\\"{}\\\" --scheduled-check", task);
    let args = [
        "/Create",
        "/TN",
        "Agent Manager - Daily Check",
        "/SC",
        "DAILY",
        "/ST",
        config.time.as_str(),
        "/TR",
        action.as_str(),
        "/F",
    ];
    let output = Command::new("schtasks.exe")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

fn run_scheduled_check() {
    let mut agents = Vec::new();
    for adapter in all_adapters() {
        if let Ok(mut agent) = adapter.discover() {
            let _ = adapter.check_latest(&mut agent);
            agents.push(agent);
        }
    }
    let _ = storage::save_agents(&agents);
}
fn main() {
    if std::env::args().any(|arg| arg == "--scheduled-check") {
        run_scheduled_check();
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(AppState { agents: Mutex::new(Vec::new()) })
        .invoke_handler(tauri::generate_handler![
            discover_agents,
            check_updates,
            update_agent,
            open_official_url,
            get_schedule,
            set_schedule
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("failed to run Agent Manager");
}







