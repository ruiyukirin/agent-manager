use super::common::*;
use crate::adapter::{AgentAdapter, not_installed};
use crate::model::{AgentInstance, AgentStatus, UpdateMode, UpdateResult};
use std::fs;
use std::path::{Path, PathBuf};

pub struct CodexAdapter;
impl AgentAdapter for CodexAdapter {
    fn id(&self) -> &'static str { "codex" }
    fn display_name(&self) -> &'static str { "Codex" }
    fn publisher(&self) -> &'static str { "OpenAI" }
    fn official_url(&self) -> &'static str { official_url_for("codex") }
    fn discover(&self) -> Result<AgentInstance, String> {
        // Step 0: trace from running process (AppX)
        if let Some(agent) = try_running_codex(self) { return Ok(agent); }

        // Step 1: WindowsApps (AppX) — MS Store install
        let windows_apps = program_files().join("WindowsApps");
        let install_path = find_child_prefix(&windows_apps, "OpenAI.Codex_")
            .or_else(|| find_child(&windows_apps, "OpenAI.Codex"));
        if let Some(install_path) = install_path {
            let executable = first_file(&vec![
                install_path.join("app\\resources\\codex.exe"),
                install_path.join("codex.exe"),
            ]);
            let display = read_registry_display_version("OpenAI.Codex")
                .or_else(|| install_path.to_string_lossy().split('_').nth(1).map(ToOwned::to_owned));
            return Ok(AgentInstance {
                id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                install_path: Some(install_path.to_string_lossy().into_owned()),
                executable_path: executable.as_ref().map(|p| p.to_string_lossy().into_owned()),
                version: Some(crate::model::VersionSnapshot {
                    product: display.clone(), component: None, bootstrap: None, channel: Some("AppX".into()), pe: display,
                }),
                latest_version: None, status: AgentStatus::Checking, running: is_process_running("codex.exe"),
                update_mode: UpdateMode::NativeUpdater, official_url: self.official_url().into(), detail: "桌面版安装".into(), last_checked: None,
            });
        }

        // Step 2: Codex CLI installation
        if let Some(cli_path) = find_codex_cli_install() {
            if let Some(executable) = first_file(&vec![cli_path.join("codex.exe")]) {
                let version = parse_version_from_path(&executable).or_else(|| version_from_file(&executable));
                return Ok(AgentInstance {
                    id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                    install_path: Some(cli_path.to_string_lossy().into_owned()),
                    executable_path: Some(executable.to_string_lossy().into_owned()),
                    version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("CLI".into()), pe: version }),
                    latest_version: None, status: AgentStatus::Checking, running: is_process_running("codex.exe"), update_mode: UpdateMode::NativeUpdater,
                    official_url: self.official_url().into(), detail: "CLI 安装".into(), last_checked: None,
                });
            }
        }

        // Step 3: Program paths fallback
        let roots = vec![local_app_data().join("Programs"), local_app_data().join("Microsoft\\WindowsApps")];
        let candidates = path_candidates(&roots, &["Codex", "codex.exe"]);
        if let Some(path) = first_existing(&candidates) {
            let executable = first_file(&vec![path.join("codex.exe"), path.join("app\\resources\\codex.exe")]);
            Ok(AgentInstance {
                id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                install_path: Some(path.to_string_lossy().into_owned()),
                executable_path: executable.map(|p| p.to_string_lossy().into_owned()),
                version: None, latest_version: None, status: AgentStatus::Checking, running: is_process_running("codex.exe"),
                update_mode: UpdateMode::NativeUpdater, official_url: self.official_url().into(), detail: "Programs 路径".into(), last_checked: None,
            })
        } else { Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 Codex 安装")) }
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        match winget_latest_version("OpenAI.Codex") {
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
                    "AppX 精确版本由 Microsoft Store 管理".into()
                } else {
                    "CLI 通过 codex update 命令更新".into()
                };
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult { success: false, message: "请通过 Microsoft Store 或 codex update 更新".into(), mode: UpdateMode::ManualAction, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None })
    }
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> {
        if was_running { if let Some(path) = agent.executable_path.as_deref() { return launch(Path::new(path)); } }
        Ok(())
    }
}

fn try_running_codex(adapter: &CodexAdapter) -> Option<AgentInstance> {
    let script = "Get-Process codex -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Path";
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output().ok()?;
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_str.is_empty() || !path_str.contains("codex.exe") { return None; }
    let path = PathBuf::from(&path_str);
    if !path.is_file() { return None; }
    let install = path.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(PathBuf::from);
    if let Some(install_dir) = install {
        let dir_name = install_dir.file_name().and_then(|n| n.to_str())?;
        if dir_name.starts_with("OpenAI.Codex_") {
            let ver = dir_name.strip_prefix("OpenAI.Codex_")
                .and_then(|s| s.split('_').next())
                .map(|s| s.to_string());
            let pe = read_pe_version(&path).or_else(|| ver.clone());
            return Some(AgentInstance {
                id: adapter.id().into(), name: "Codex".into(), publisher: "OpenAI".into(), installed: true,
                install_path: Some(install_dir.to_string_lossy().into_owned()),
                executable_path: Some(path.to_string_lossy().into_owned()),
                version: Some(crate::model::VersionSnapshot { product: ver.clone(), component: None, bootstrap: None, channel: Some("AppX".into()), pe }),
                latest_version: None, status: AgentStatus::Checking, running: true,
                update_mode: UpdateMode::NativeUpdater,
                official_url: adapter.official_url().into(), detail: "桌面版安装".into(), last_checked: None,
            });
        }
    }
    None
}

fn find_codex_cli_install() -> Option<PathBuf> {
    let codex_root = local_app_data().join("OpenAI\\Codex\\bin");
    if !codex_root.exists() { return None; }
    let entries = fs::read_dir(&codex_root).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let exe = p.join("codex.exe");
            if exe.is_file() { return Some(p); }
        }
    }
    None
}

pub struct HermesAdapter;
impl AgentAdapter for HermesAdapter {
    fn id(&self) -> &'static str { "hermes" }
    fn display_name(&self) -> &'static str { "Hermes Agent" }
    fn publisher(&self) -> &'static str { "Nous Research" }
    fn official_url(&self) -> &'static str { official_url_for("hermes") }
    fn discover(&self) -> Result<AgentInstance, String> {
        if let Some(hermes_exe) = find_hermes_exe() {
            let install = hermes_exe.parent().and_then(|p| p.parent()).map(PathBuf::from);
            let version = command_capture("hermes.exe", &["--version"]).and_then(|s| s.lines().next().map(|l| l.trim().to_string()));
            return Ok(AgentInstance {
                id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
                install_path: install.map(|p| p.to_string_lossy().into_owned()),
                executable_path: Some(hermes_exe.to_string_lossy().into_owned()),
                version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("installer".into()), pe: version }),
                latest_version: None, status: AgentStatus::Checking, running: is_process_running("hermes.exe"), update_mode: UpdateMode::NativeUpdater,
                official_url: self.official_url().into(), detail: "原生 Windows 安装".into(), last_checked: None,
            });
        }
        let candidates = vec![local_app_data().join("hermes\\hermes-agent\\bin\\hermes.exe"), local_app_data().join("hermes\\bin\\hermes.exe"), PathBuf::from("C:\\Users\\Public\\.hermes\\bin\\hermes.exe")];
        if let Some(path) = first_file(&candidates) {
            let install = path.parent().and_then(|p| p.parent()).map(PathBuf::from);
            let version = command_capture("hermes.exe", &["--version"]).and_then(|s| s.lines().next().map(|l| l.trim().to_string()));
            Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: install.map(|p| p.to_string_lossy().into_owned()), executable_path: Some(path.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("installer".into()), pe: version }), latest_version: None, status: AgentStatus::Checking, running: is_process_running("hermes.exe"), update_mode: UpdateMode::NativeUpdater, official_url: self.official_url().into(), detail: "原生 Windows 安装".into(), last_checked: None })
        } else { Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 Hermes 安装")) }
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        match github_latest_version("NousResearch", "hermes-agent") {
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
                agent.status = AgentStatus::Error;
                agent.detail = "无法查询 GitHub 最新版本".into();
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }
    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> { Ok(UpdateResult { success: false, message: "请运行 hermes update 完成更新".into(), mode: UpdateMode::NativeUpdater, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None }) }
    fn restart_if_was_running(&self, _agent: &mut AgentInstance, was_running: bool) -> Result<(), String> { if was_running { let _ = command_capture("hermes.exe", &["gateway", "restart"]); } Ok(()) }
}

pub struct OpenClawAdapter;
impl AgentAdapter for OpenClawAdapter {
    fn id(&self) -> &'static str { "openclaw" }
    fn display_name(&self) -> &'static str { "OpenClaw" }
    fn publisher(&self) -> &'static str { "OpenClaw Foundation" }
    fn official_url(&self) -> &'static str { official_url_for("openclaw") }
    fn discover(&self) -> Result<AgentInstance, String> {
        let roots = vec![local_app_data().join("npm"), PathBuf::from("C:\\Users\\Public\\.npm-global")];
        if let Some(path) = first_file(&path_candidates(&roots, &["openclaw.cmd", "openclaw.exe"])) {
            let version = command_capture("openclaw.cmd", &["--version"]).and_then(|s| s.lines().next().map(|l| l.trim().to_string()));
            Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: path.parent().map(|p| p.to_string_lossy().into_owned()), executable_path: Some(path.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("npm".into()), pe: version }), latest_version: None, status: AgentStatus::Checking, running: is_process_running("openclaw.exe") || is_process_running("openclaw-gateway.exe"), update_mode: UpdateMode::NativeUpdater, official_url: self.official_url().into(), detail: "npm / pnpm 安装".into(), last_checked: None })
        } else { Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 OpenClaw 安装")) }
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        match npm_latest_version("openclaw") {
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
                agent.detail = "OpenClaw 使用 openclaw update status --json 检查更新".into();
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }
    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> { Ok(UpdateResult { success: false, message: "请运行 openclaw update 完成更新".into(), mode: UpdateMode::NativeUpdater, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None }) }
    fn restart_if_was_running(&self, _agent: &mut AgentInstance, was_running: bool) -> Result<(), String> { if was_running { let _ = command_capture("openclaw.cmd", &["gateway", "restart"]); } Ok(()) }
}

fn find_hermes_exe() -> Option<PathBuf> {
    let hermes_root = local_app_data().join("hermes\\hermes-agent");
    if !hermes_root.exists() { return None; }
    let mut stack = vec![hermes_root];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() { stack.push(p); }
                else if p.is_file() && p.file_name().and_then(|n| n.to_str()) == Some("hermes.exe") {
                    return Some(p);
                }
            }
        }
    }
    None
}
