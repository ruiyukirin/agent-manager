// Author: Kirin
use agent_manager::model::AgentInstance;
use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};

fn backup_root(agent_id: &str) -> std::io::Result<std::path::PathBuf> {
    let root = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:\\Users\\Public\\AppData\\Roaming"));
    let path = root.join("AgentManager").join("backups").join(agent_id);
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// 凭据类文件不进备份目录 —— 备份会保留多份，明文密钥不该被复制进去。
fn is_secret(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["credential", "secret", "token", "password", ".key", ".pem", "cookie"].iter().any(|needle| lower.contains(needle))
}

fn copy_if_exists(from: &Path, to: &Path) {
    if from.file_name().map(|name| is_secret(name.to_string_lossy().as_ref())).unwrap_or(false) { return; }
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
        let name = entry.file_name();
        if is_secret(name.to_string_lossy().as_ref()) { continue; }
        let target = to.join(&name);
        if entry.path().is_dir() { copy_dir(&entry.path(), &target)?; } else { fs::copy(entry.path(), &target)?; }
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
    }
    // 刻意不备份可执行程序：GUI 程序动辄几百 MB，且随时能从官方重新下载。
    if let Some(app_data) = std::env::var_os("APPDATA").map(std::path::PathBuf::from) {
        for name in ["Tencent\\WorkBuddy", "WorkBuddy"] {
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

#[cfg(test)]
mod tests {
    use super::is_secret;

    #[test]
    fn never_backs_up_credentials() {
        // 备份会保留多份，明文凭据绝不能进备份目录（~/.dsh/.credentials.yaml 是真实存在的例子）。
        assert!(is_secret(".credentials.yaml"));
        assert!(is_secret("access_token.json"));
        assert!(is_secret("server.pem"));
        assert!(is_secret("id_rsa.key"));
        assert!(is_secret("cookie.txt"));
        assert!(!is_secret("settings.json"));
        assert!(!is_secret("WorkBuddy.exe"));
    }

    #[test]
    fn copy_dir_skips_secret_files() {
        let base = std::env::temp_dir().join(format!("agent-manager-backup-test-{}", std::process::id()));
        let from = base.join("from");
        let to = base.join("to");
        std::fs::create_dir_all(from.join("sub")).unwrap();
        std::fs::write(from.join("settings.json"), b"{}").unwrap();
        std::fs::write(from.join(".credentials.yaml"), b"should-not-be-copied").unwrap();
        std::fs::write(from.join("sub").join("log.txt"), b"ok").unwrap();
        std::fs::write(from.join("sub").join("access_token.json"), b"should-not-be-copied").unwrap();

        super::copy_dir(&from, &to).unwrap();

        assert!(to.join("settings.json").exists());
        assert!(to.join("sub").join("log.txt").exists());
        assert!(!to.join(".credentials.yaml").exists(), "凭据文件不该被备份");
        assert!(!to.join("sub").join("access_token.json").exists(), "凭据文件不该被备份");
        let _ = std::fs::remove_dir_all(&base);
    }
}

