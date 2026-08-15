use super::common::*;
use crate::adapter::{AgentAdapter, not_installed};
use crate::model::{AgentInstance, AgentStatus, UpdateMode, UpdateResult};
use std::{path::{Path, PathBuf}};

pub struct WorkBuddyAdapter;
impl AgentAdapter for WorkBuddyAdapter {
    fn id(&self) -> &'static str { "workbuddy" }
    fn display_name(&self) -> &'static str { "WorkBuddy" }
    fn publisher(&self) -> &'static str { "Tencent" }
    fn official_url(&self) -> &'static str { official_url_for("workbuddy") }
    fn discover(&self) -> Result<AgentInstance, String> {
        if let Some(install) = read_registry_install_location("WorkBuddy") {
            if let Some(executable) = first_file(&vec![install.join("WorkBuddy.exe"), install.join("Application\\WorkBuddy.exe")]) {
                let version = version_from_file(&executable);
                return Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: Some(install.to_string_lossy().into_owned()), executable_path: Some(executable.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("Windows x64".into()), pe: version }), latest_version: None, status: AgentStatus::Checking, running: is_process_running("WorkBuddy.exe"), update_mode: UpdateMode::ManualAction, official_url: self.official_url().into(), detail: "Windows 客户端已识别".into(), last_checked: None });
            }
        }
        let roots = vec![program_files().join("Tencent"), local_app_data().join("Tencent"), local_app_data().join("Programs"), app_data().join("Tencent"), app_data().join("Microsoft\\Windows\\Start Menu\\Programs")];
        let names = ["WorkBuddy", "WorkBuddy.exe", "WorkBuddy\\WorkBuddy.exe", "WorkBuddy\\Application\\WorkBuddy.exe", "CodeBuddy\\WorkBuddy.exe"];
        let paths = path_candidates(&roots, &names);
        let path = first_file(&paths).or_else(|| roots.iter().filter_map(|root| find_child(root, "WorkBuddy.exe")).next());
        if let Some(path) = path {
            let install = path.parent().and_then(|p| p.parent()).map(Path::to_path_buf).or_else(|| Some(path.parent().unwrap_or(&path).to_path_buf()));
            let version = version_from_file(&path);
            Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: install.map(|p| p.to_string_lossy().into_owned()), executable_path: Some(path.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("Windows x64".into()), pe: version.clone() }), latest_version: None, status: AgentStatus::Checking, running: is_process_running("WorkBuddy.exe"), update_mode: UpdateMode::ManualAction, official_url: self.official_url().into(), detail: if version.is_some() { "Windows 客户端已识别" } else { "Windows 客户端路径已识别，版本需由安装包确认" }.into(), last_checked: None })
        } else { Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 WorkBuddy 安装")) }
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        agent.latest_version = None;
        agent.status = AgentStatus::ManualAction;
        agent.detail = "官方客户端未暴露独立版本接口；请使用官方安装包".into();
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }
    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult { success: false, message: "WorkBuddy 的静默安装参数尚未在本机验证，请打开官方入口获取最新客户端".into(), mode: UpdateMode::ManualAction, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None })
    }
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> { if was_running { if let Some(path) = agent.executable_path.as_deref() { return launch(Path::new(path)); } } Ok(()) }
}

pub struct MarvisAdapter;
impl AgentAdapter for MarvisAdapter {
    fn id(&self) -> &'static str { "marvis" }
    fn display_name(&self) -> &'static str { "Marvis" }
    fn publisher(&self) -> &'static str { "Tencent" }
    fn official_url(&self) -> &'static str { official_url_for("marvis") }
    fn discover(&self) -> Result<AgentInstance, String> {
        let program_root = read_registry_install_location("Marvis").unwrap_or_else(|| program_files().join("Tencent\\Marvis"));
        let launcher = first_file(&vec![program_root.join("MarvisLauncher.exe"), program_root.join("Application\\MarvisLauncher.exe")]);
        if !program_root.exists() && launcher.is_none() { return Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 Marvis 安装")); }
        let setup_dirs = program_root.join("Application").read_dir().ok().into_iter().flatten().filter_map(|entry| entry.ok().map(|e| e.path())).filter(|path| path.is_dir() && path.join("Setup.json").exists()).collect::<Vec<PathBuf>>();
        let setup = setup_dirs.last();
        let component = setup.and_then(|path| parse_setup_version(&path.join("Setup.json")));
        let product = read_registry_display_version("Marvis").or_else(|| component.clone());
        let executable = setup.as_ref().and_then(|path| first_file(&vec![path.join("MarvisLauncher.exe"), path.join("Marvis.exe"), path.join("MarvisUpdate.exe")])).or(launcher);
        let pe_version = setup.as_ref().and_then(|path| read_pe_version(&path.join("Marvis.exe")));
        let running = is_process_running("Marvis.exe") || is_process_running("MarvisLauncher.exe") || is_process_running("MarvisSvr.exe");
        Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: Some(program_root.to_string_lossy().into_owned()), executable_path: executable.map(|p| p.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product, component, bootstrap: None, channel: Some("Windows".into()), pe: pe_version }), latest_version: None, status: AgentStatus::Checking, running, update_mode: UpdateMode::ManualAction, official_url: self.official_url().into(), detail: if setup.is_some() { "检测到 MarvisUpdate.exe 和版本清单" } else { "检测到 Marvis 启动器" }.into(), last_checked: None })
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        agent.latest_version = None;
        match download_to_temp(self.official_url(), "marvis_bootstrap") {
            Ok(path) => { let bootstrap = parse_version_from_path(&path); let hash = sha256(&path); agent.status = AgentStatus::ManualAction; agent.detail = if bootstrap.is_some() { format!("已获取官方引导包 {}，但这不是应用最新版本；请使用 Marvis 内置更新器确认", bootstrap.unwrap_or_default()) } else { "已获取官方引导包，但无法解析应用版本".into() }; if let Some(hash) = hash { agent.detail.push_str(&format!(" · {}", &hash[..12])); }  }
            Err(error) => { agent.status = AgentStatus::Error; agent.detail = error; }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }
    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult { success: false, message: "Marvis 官方未提供可直接读取的精确应用版本接口；请通过 MarvisUpdate.exe 或官方安装器更新".into(), mode: UpdateMode::ManualAction, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None })
    }
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> { if was_running { let launcher = program_files().join("Tencent\\Marvis\\MarvisLauncher.exe"); let path = agent.executable_path.as_deref().map(Path::new).unwrap_or(&launcher); return launch(path); } Ok(()) }
}










