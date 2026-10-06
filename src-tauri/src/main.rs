// Author: Kirin
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use agent_manager::adapter::{adapter_for, all_adapters, not_installed};
use agent_manager::adapters::winget_install;
use agent_manager::model::{AgentInstance, AgentStatus, InstallMethod, InstallResult, ScheduleConfig, UpdateResult};
use tauri::Manager;

mod backup;
mod state;
mod storage;
use state::AppState;

#[tauri::command]
fn discover_agents(state: tauri::State<AppState>) -> Result<Vec<AgentInstance>, String> {
    let mut agents = Vec::new();
    for adapter in all_adapters() {
        match adapter.discover() {
            Ok(agent) => agents.push(agent),
            Err(error) => {
                let mut agent = not_installed(adapter.id(), adapter.display_name(), adapter.publisher(), adapter.official_url(), &format!("检测失败：{}", error));
                agent.status = AgentStatus::Error;
                agents.push(agent);
            }
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
    let adapter = adapter_for(&agent_id).ok_or_else(|| format!("没有适配器: {}", agent_id))?;
    // 只有会真正改动本机文件的更新才值得先备份；当前所有适配器都是提示用户手动更新，不必白拷一份。
    if adapter.update_mutates_files() {
        let _ = backup::backup_before_update(agent);
    }
    let mut result = adapter.update(agent)?;
    result.previous_version = previous;
    if result.success && was_running {
        adapter.restart_if_was_running(agent, was_running)?;
    }
    storage::save_agents(&*agents)?;
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
        let output = Command::new("schtasks.exe").args(["/Delete", "/TN", "Agent Manager - Daily Check", "/F"]).output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
            if !stderr.contains("cannot find") && !stderr.contains("does not exist") {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
            }
        }
        storage::save_schedule(&config)?;
        return Ok(());
    }
    storage::save_schedule(&config)?;
    let task = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    let action = format!("\"{}\" --scheduled-check", task);
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

fn run_scheduled_check(identifier: &str) {
    let mut agents = Vec::new();
    let mut updates = 0usize;
    let mut errors = 0usize;
    for adapter in all_adapters() {
        match adapter.discover() {
            Ok(mut agent) => {
                if adapter.check_latest(&mut agent).is_ok() && agent.status == AgentStatus::UpdateAvailable {
                    updates += 1;
                }
                agents.push(agent);
            }
            Err(_) => {
                errors += 1;
            }
        }
    }
    let _ = storage::save_agents(&agents);

    let message = if updates > 0 {
        format!("检测到 {} 个 Agent 有可用更新", updates)
    } else if errors > 0 {
        format!("定时检查完成，{} 个 Agent 检测失败", errors)
    } else {
        "定时检查完成，所有 Agent 均为最新版本".to_string()
    };
    show_scheduled_notification(identifier, &message);
}

fn show_scheduled_notification(identifier: &str, message: &str) {
    let mut notification = notify_rust::Notification::new();
    notification.summary("Agent Manager");
    notification.body(message);

    let is_dev = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.to_string_lossy().to_ascii_lowercase()))
        .map(|dir| dir.ends_with(r"target\debug") || dir.ends_with(r"target\release"))
        .unwrap_or(false);
    if !is_dev {
        notification.app_id(identifier);
    }
    let _ = notification.show();
}

/// Download an installer to the chosen directory. Returns the downloaded file path.
#[tauri::command]
fn download_agent(agent_id: String, download_dir: String, state: tauri::State<AppState>) -> Result<String, String> {
    let agents = state.agents.lock().map_err(|e| e.to_string())?;
    let agent = agents.iter().find(|a| a.id == agent_id).ok_or_else(|| format!("未找到 Agent: {}", agent_id))?;
    if agent.installed { return Err("该 Agent 已安装".into()); }
    let adapter = adapter_for(&agent_id).ok_or_else(|| format!("没有适配器: {}", agent_id))?;
    let url = match adapter.install_method() {
        InstallMethod::DirectDownload { url } => url,
        _ => return Err("此 Agent 暂不支持直接下载安装，请使用其他安装方式".into()),
    };
    drop(agents);

    let dl_dir = PathBuf::from(&download_dir);
    if !dl_dir.exists() { return Err("下载目录不存在".into()); }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis();
    let filename = match url.rsplit(|c: char| c == '/' || c == '\\' || c == '?').next() {
        Some(name) if name.ends_with(".exe") => name,
        _ => "installer.exe",
    };
    let output_name = format!("{}_{}_{}", agent_id, stamp, filename);
    let output_path = dl_dir.join(&output_name);

    let result = Command::new("curl.exe").args(["-fL", "--connect-timeout", "15", "--max-time", "120", &url, "-o"]).arg(&output_path).output().map_err(|e| format!("无法启动 curl: {e}"))?;
    if !result.status.success() {
        let _ = std::fs::remove_file(&output_path);
        return Err(format!("下载失败: {}", String::from_utf8_lossy(&result.stderr).trim()));
    }
    let bytes = std::fs::read(&output_path).map_err(|e| e.to_string())?;
    if bytes.len() < 2 || &bytes[..2] != b"MZ" {
        let _ = std::fs::remove_file(&output_path);
        return Err("下载的文件不是有效的 Windows 安装包".to_string());
    }
    Ok(output_path.to_string_lossy().to_string())
}

fn replace_agent_and_save(state: &tauri::State<AppState>, agent: AgentInstance) -> Result<(), String> {
    let mut agents = state.agents.lock().map_err(|e| e.to_string())?;
    if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
        *existing = agent;
    } else {
        agents.push(agent);
    }
    storage::save_agents(&*agents)
}

/// Install a previously downloaded installer to the chosen directory.
#[tauri::command]
fn install_agent(agent_id: String, installer_path: String, install_dir: String, state: tauri::State<AppState>) -> Result<InstallResult, String> {
    let adapter = adapter_for(&agent_id).ok_or_else(|| format!("没有适配器: {}", agent_id))?;
    let method = adapter.install_method();

    let direct_url = match method {
        InstallMethod::Winget { package_id } => {
            winget_install(&package_id)?;
            let refreshed = adapter.discover().map_err(|e| format!("winget 安装命令已执行，但重新检测失败：{}", e))?;
            let message = if refreshed.installed { format!("{} 安装完成", refreshed.name) } else { format!("{} 安装命令已执行，请重新扫描确认", refreshed.name) };
            replace_agent_and_save(&state, refreshed)?;
            return Ok(InstallResult { success: true, message, method: InstallMethod::Winget { package_id }, needs_restart: false });
        }
        InstallMethod::DirectDownload { url } => url,
        InstallMethod::OpenBrowser { .. } => {
            return Err("此 Agent 不使用 install_agent 安装，请使用官方入口或直接下载流程".into());
        }
    };

    let path = PathBuf::from(&installer_path);
    if !path.exists() { return Err("安装文件不存在".into()); }

    let target_dir = PathBuf::from(&install_dir);
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("无法创建安装目录: {e}"))?;

    // Try common silent install flags in priority order
    let silent_flags: &[&[&str]] = &[
        &["/S", &format!("/D={}", install_dir)],
        &["/VERYSILENT", &format!("/DIR={}", install_dir)],
        &["/SILENT", &format!("/DIR={}", install_dir)],
        &["/quiet"],
        &["/silent"],
    ];
    for args in silent_flags {
        if let Ok(status) = Command::new(&path).args(*args).status() {
            if status.success() {
                let refreshed = adapter.discover().map_err(|e| format!("安装命令已执行，但重新检测失败：{}", e))?;
                let message = if refreshed.installed { format!("{} 安装完成", refreshed.name) } else { format!("{} 安装命令已执行，请重新扫描确认", refreshed.name) };
                replace_agent_and_save(&state, refreshed)?;
                return Ok(InstallResult { success: true, message, method: InstallMethod::DirectDownload { url: direct_url }, needs_restart: false });
            }
        }
    }
    Err(format!("静默安装失败，安装文件保存在：{}。请手动安装。", path.display()))
}

fn main() {
    let context = tauri::generate_context!();
    if std::env::args().any(|arg| arg == "--scheduled-check") {
        run_scheduled_check(&context.config().identifier);
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { agents: Mutex::new(Vec::new()) })
        .invoke_handler(tauri::generate_handler![
            discover_agents,
            check_updates,
            update_agent,
            install_agent,
            download_agent,
            open_official_url,
            get_schedule,
            set_schedule
        ])
        .setup(|app| {
            eprintln!("[setup] Agent Manager started");
            // Try to get existing window and force-set geometry
            if let Some(w) = app.get_webview_window("main") {
                eprintln!("[setup] got window, setting geometry...");
                let _ = w.set_position(tauri::PhysicalPosition::new(100, 100));
                let _ = w.set_size(tauri::PhysicalSize::new(1440, 900));
                let _ = w.set_min_size(Some(tauri::PhysicalSize::new(1120, 720)));
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        })
        .run(context)
        .expect("failed to run Agent Manager");
}
