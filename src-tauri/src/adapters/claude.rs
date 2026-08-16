use super::common::*;
use crate::adapter::{AgentAdapter, not_installed};
use crate::model::{AgentInstance, AgentStatus, UpdateMode, UpdateResult};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ClaudeAdapter;

impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &'static str { "claude" }
    fn display_name(&self) -> &'static str { "Claude" }
    fn publisher(&self) -> &'static str { "Anthropic" }
    fn official_url(&self) -> &'static str { official_url_for("claude") }

    fn discover(&self) -> Result<AgentInstance, String> {
        // Step 0: try WindowsApps (AppX)
        if let Some(agent) = try_windows_apps_claude(self) { return Ok(agent); }
        // Step 0b: trace claude.exe from running process
        if let Some(agent) = try_running_process(self) { return Ok(agent); }

        // Step 1: registry
        if let Some(install) = read_registry_install_location("Claude") {
            if let Some(executable) = first_file(&vec![install.join("claude.exe"), install.join("Claude.exe")]) {
                let version = version_from_file(&executable);
                return Ok(AgentInstance {
                    id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                    install_path: Some(install.to_string_lossy().into_owned()), executable_path: Some(executable.to_string_lossy().into_owned()),
                    version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("Windows".into()), pe: version }),
                    latest_version: None, status: AgentStatus::Checking, running: is_process_running("claude.exe"), update_mode: UpdateMode::ManualAction,
                    official_url: self.official_url().into(), detail: "found via registry".into(), last_checked: None,
                });
            }
        }

        // Step 2: known paths
        let roots = vec![
            local_app_data().join("Claude-3p\\claude-code"),
            local_app_data().join("Claude-3p"),
            local_app_data().join("Programs\\Claude"),
            local_app_data().join("Programs"),
            program_files().join("Claude"),
            app_data().join("npm"),
            PathBuf::from("C:\\Users\\Public\\.npm-global"),
        ];
        let names = ["claude.exe", "Claude.exe", "claude.cmd"];
        let candidates = path_candidates(&roots, &names);
        if let Some(path) = first_file(&candidates) {
            let install = path.parent().and_then(|p| p.parent()).map(Path::to_path_buf).or_else(|| Some(path.parent().unwrap_or(&path).to_path_buf()));
            let version = parse_version_from_path(&path).or_else(|| version_from_file(&path));
            return Ok(AgentInstance {
                id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                install_path: install.map(|p| p.to_string_lossy().into_owned()),
                executable_path: Some(path.to_string_lossy().into_owned()),
                version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("native".into()), pe: version.clone() }),
                latest_version: None, status: AgentStatus::Checking, running: is_process_running("claude.exe"), update_mode: UpdateMode::ManualAction,
                official_url: self.official_url().into(), detail: if version.is_some() { "found Claude Code install".into() } else { "found Claude entry".into() }, last_checked: None,
            });
        }

        // Step 3: recursive scan
        fn scan_dir_recursive(dir: &Path, depth: u32) -> Option<PathBuf> {
            if depth == 0 { return None; }
            let entries = fs::read_dir(dir).ok()?;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let exe = p.join("claude.exe");
                    if exe.is_file() { return Some(exe); }
                    if let Some(found) = scan_dir_recursive(&p, depth - 1) { return Some(found); }
                }
            }
            None
        }
        let claude_roots = vec![local_app_data().join("Claude-3p"), PathBuf::from("C:\\Users\\yuqil\\.claude")];
        for root in &claude_roots {
            if root.exists() {
                if let Some(exe) = scan_dir_recursive(root, 3) {
                    let install = exe.parent().map(PathBuf::from);
                    let version = parse_version_from_path(&exe).or_else(|| version_from_file(&exe));
                    return Ok(AgentInstance {
                        id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                        install_path: install.as_ref().map(|p| p.to_string_lossy().into_owned()),
                        executable_path: Some(exe.to_string_lossy().into_owned()),
                        version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("native".into()), pe: version.clone() }),
                        latest_version: None, status: AgentStatus::Checking, running: is_process_running("claude.exe"), update_mode: UpdateMode::ManualAction,
                        official_url: self.official_url().into(), detail: if version.is_some() { "found via deep scan".into() } else { "found Claude entry".into() }, last_checked: None,
                    });
                }
            }
        }

        Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 Claude 安装"))
    }

    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        match winget_latest_version("Anthropic.Claude") {
            Some(latest) => {
                agent.latest_version = Some(latest.clone());
                let current = agent.version.as_ref().and_then(|v| v.product.as_deref().or(v.pe.as_deref())).unwrap_or("");
                if current.is_empty() {
                    agent.status = AgentStatus::ManualAction;
                    agent.detail = format!("云端最新版本：{}", latest);
                } else if is_update_available(current, &latest) {
                    agent.status = AgentStatus::UpdateAvailable;
                    agent.detail = format!("{} → {} 可更新", current, latest);
                } else {
                    agent.status = AgentStatus::UpToDate;
                    agent.detail = "已是最新版本".into();
                }
            }
            None => {
                agent.latest_version = None;
                agent.status = AgentStatus::ManualAction;
                agent.detail = if agent.install_path.as_deref().map_or(false, |p| p.contains("WindowsApps")) {
                    "Claude 桌面版通过 Microsoft Store 更新".into()
                } else {
                    "Claude Code 通过 claude update 或官网下载更新".into()
                };
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult {
            success: false,
            message: "请通过 claude update 或官网下载更新".into(),
            mode: UpdateMode::ManualAction,
            official_url: Some(self.official_url().into()),
            needs_restart: false,
            previous_version: None,
            current_version: None,
        })
    }

    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> {
        if was_running { if let Some(path) = agent.executable_path.as_deref() { return launch(Path::new(path)); } }
        Ok(())
    }
}

fn try_windows_apps_claude(adapter: &ClaudeAdapter) -> Option<AgentInstance> {
    let windows_apps = program_files().join("WindowsApps");
    if !windows_apps.exists() { return None; }
    let entries = fs::read_dir(&windows_apps).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str())?;
        if !name.starts_with("Claude_") { continue; }
        let executable = first_file(&vec![p.join("app\\Claude.exe"), p.join("Claude.exe"), p.join("claude.exe")])?;
        let version_raw = name.strip_prefix("Claude_").and_then(|s| s.split('_').next()).map(|s| s.to_string());
        let pe_version = read_pe_version(&executable).or_else(|| version_raw.clone());
        return Some(AgentInstance {
            id: adapter.id().into(), name: "Claude".into(), publisher: "Anthropic".into(), installed: true,
            install_path: Some(p.to_string_lossy().into_owned()),
            executable_path: Some(executable.to_string_lossy().into_owned()),
            version: Some(crate::model::VersionSnapshot { product: version_raw.clone(), component: None, bootstrap: None, channel: Some("AppX".into()), pe: pe_version }),
            latest_version: None, status: AgentStatus::Checking, running: is_process_running("claude.exe"), update_mode: UpdateMode::ManualAction,
            official_url: adapter.official_url().into(), detail: "桌面版安装".into(), last_checked: None,
        });
    }
    None
}

fn try_running_process(adapter: &ClaudeAdapter) -> Option<AgentInstance> {
    let script = "Get-Process claude -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Path";
    let output = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", script]).output().ok()?;
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_str.is_empty() || !path_str.contains("Claude.exe") { return None; }
    let path = PathBuf::from(&path_str);
    if !path.is_file() { return None; }
    let install = path.parent().and_then(|p| p.parent()).map(PathBuf::from);
    if let Some(install_dir) = install {
        let dir_name = install_dir.file_name().and_then(|n| n.to_str())?;
        if dir_name.starts_with("Claude_") {
            let version_raw = dir_name.strip_prefix("Claude_").and_then(|s| s.split('_').next()).map(|s| s.to_string());
            let pe_version = read_pe_version(&path).or_else(|| version_raw.clone());
            return Some(AgentInstance {
                id: adapter.id().into(), name: "Claude".into(), publisher: "Anthropic".into(), installed: true,
                install_path: Some(install_dir.to_string_lossy().into_owned()),
                executable_path: Some(path.to_string_lossy().into_owned()),
                version: Some(crate::model::VersionSnapshot { product: version_raw.clone(), component: None, bootstrap: None, channel: Some("AppX".into()), pe: pe_version }),
                latest_version: None, status: AgentStatus::Checking, running: true, update_mode: UpdateMode::ManualAction,
                official_url: adapter.official_url().into(), detail: "桌面版安装".into(), last_checked: None,
            });
        }
    }
    None
}
