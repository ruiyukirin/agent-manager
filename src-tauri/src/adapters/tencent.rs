// Author: Kirin
use super::common::*;
use crate::adapter::{AgentAdapter, not_installed};
use crate::model::{AgentInstance, AgentStatus, InstallMethod, UpdateMode, UpdateResult};
use std::path::Path;

pub struct WorkBuddyAdapter;
impl AgentAdapter for WorkBuddyAdapter {
    fn id(&self) -> &'static str { "workbuddy" }
    fn display_name(&self) -> &'static str { "WorkBuddy" }
    fn publisher(&self) -> &'static str { "Tencent" }
    fn official_url(&self) -> &'static str { official_url_for("workbuddy") }
    fn install_method(&self) -> InstallMethod { InstallMethod::OpenBrowser { url: self.official_url().into() } }
    fn install(&self) -> Result<(), String> { Err("WorkBuddy 请前往 workbuddy.ai 下载安装".into()) }
    fn discover(&self) -> Result<AgentInstance, String> {
        // 卸载表里有 WorkBuddy 的版本，但不一定给 InstallLocation（本机就是空的），所以位置还得靠找。
        let entry = uninstall_entry("WorkBuddy");
        let declared = entry.as_ref().and_then(|(version, _)| version.clone());
        let registry_root = entry.as_ref().and_then(|(_, location)| location.clone());

        let executable = registry_root.as_ref()
            .and_then(|root| first_file(&vec![root.join("WorkBuddy.exe"), root.join("Application\\WorkBuddy.exe")]))
            .or_else(|| {
                let roots = vec![program_files().join("Tencent"), local_app_data().join("Tencent"), local_app_data().join("Programs"), app_data().join("Tencent"), app_data().join("Microsoft\\Windows\\Start Menu\\Programs")];
                let names = ["WorkBuddy", "WorkBuddy.exe", "WorkBuddy\\WorkBuddy.exe", "WorkBuddy\\Application\\WorkBuddy.exe", "CodeBuddy\\WorkBuddy.exe"];
                let paths = path_candidates(&roots, &names);
                first_file(&paths).or_else(|| roots.iter().filter_map(|root| find_child(root, "WorkBuddy.exe")).next())
            });

        let Some(executable) = executable else {
            return Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 WorkBuddy 安装"));
        };

        // WorkBuddy 是 GUI 程序，不能执行 --version（会弹出窗口），版本按「卸载表声明 → 内嵌版本 → 路径」取。
        let declared_candidates = [declared];
        let version = probe_version(&VersionProbe { cli: None, declared: &declared_candidates, pe: Some(&executable), path_hint: Some(&executable) });
        let install = registry_root.unwrap_or_else(|| install_root_for(&executable, "workbuddy"));
        Ok(AgentInstance { id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true, install_path: Some(install.to_string_lossy().into_owned()), executable_path: Some(executable.to_string_lossy().into_owned()), version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("Windows x64".into()), pe: version.clone() }), latest_version: None, status: AgentStatus::Checking, running: is_process_running("WorkBuddy.exe"), update_mode: UpdateMode::ManualAction, official_url: self.official_url().into(), install_url: self.official_url().into(), install_method: InstallMethod::OpenBrowser { url: self.official_url().into() }, detail: if version.is_some() { "Windows 客户端已识别" } else { "Windows 客户端路径已识别，版本需由安装包确认" }.into(), last_checked: None })
    }
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        match winget_latest_version("Tencent.WorkBuddy") {
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
                agent.detail = "官方客户端未暴露独立版本接口；请使用官方安装包".into();
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }
    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult { success: false, message: "WorkBuddy 的静默安装参数尚未在本机验证，请打开官方入口获取最新客户端".into(), mode: UpdateMode::ManualAction, official_url: Some(self.official_url().into()), needs_restart: false, previous_version: None, current_version: None })
    }
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> { if was_running { if let Some(path) = agent.executable_path.as_deref() { return launch(Path::new(path)); } } Ok(()) }
}
