use agent_manager::model::AgentInstance;
use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};

fn backup_root(agent_id: &str) -> std::io::Result<std::path::PathBuf> {
    let root = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:\\Users\\Public\\AppData\\Roaming"));
    let path = root.join("AgentManager").join("backups").join(agent_id);
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn copy_if_exists(from: &Path, to: &Path) {
    if from.is_file() {
        if let Some(parent) = to.parent() { let _ = fs::create_dir_all(parent); }
        let _ = fs::copy(from, to);
    } else if from.is_dir() {
        let _ = copy_dir(from, to);
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() { copy_dir(&source, &target)?; } else { fs::copy(&source, &target)?; }
    }
    Ok(())
}

pub fn create_backup(agent: &AgentInstance) -> Result<String, String> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis();
    let root = backup_root(&agent.id).map_err(|e| e.to_string())?.join(stamp.to_string());
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    if let Some(path) = &agent.install_path {
        let install = Path::new(path);
        if install.is_file() { copy_if_exists(install, &root.join("install")); }
        for name in ["Setup.json", "file_config.json", "MarvisUpdate.log", "Marvis.bak.log"] { copy_if_exists(&install.join(name), &root.join("install-metadata").join(name)); }
    }
    if let Some(path) = &agent.executable_path { copy_if_exists(Path::new(path), &root.join("executable")); }
    if let Some(app_data) = std::env::var_os("APPDATA").map(std::path::PathBuf::from) {
        for name in ["Tencent\\WorkBuddy", "Tencent\\Marvis", "WorkBuddy", "Marvis"] {
            copy_if_exists(&app_data.join(name), &root.join("data").join(name));
        }
    }
    let manifest = serde_json::json!({ "agentId": agent.id, "createdAt": stamp, "version": agent.version, "installPath": agent.install_path, "executablePath": agent.executable_path });
    fs::write(root.join("manifest.json"), serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(root.to_string_lossy().into_owned())
}

pub fn retain_recent(agent_id: &str, keep: usize) -> Result<(), String> {
    let root = backup_root(agent_id).map_err(|e| e.to_string())?;
    let mut entries = fs::read_dir(&root).map_err(|e| e.to_string())?.filter_map(|entry| entry.ok()).filter(|entry| entry.path().is_dir()).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let remove_count = entries.len().saturating_sub(keep);
    for entry in entries.into_iter().take(remove_count) { let _ = fs::remove_dir_all(entry.path()); }
    Ok(())
}

pub fn backup_before_update(agent: &AgentInstance) -> Result<String, String> {
    let result = create_backup(agent);
    let _ = retain_recent(&agent.id, 5);
    result
}

