use crate::adapters;
use crate::model::{AgentInstance, AgentStatus, InstallMethod, UpdateMode, UpdateResult};

pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn publisher(&self) -> &'static str;
    fn official_url(&self) -> &'static str;
    fn install_method(&self) -> InstallMethod;
    fn discover(&self) -> Result<AgentInstance, String>;
    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String>;
    fn update(&self, agent: &mut AgentInstance) -> Result<UpdateResult, String>;
    fn install(&self) -> Result<(), String>;
    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String>;
    /// 该适配器的 update() 是否会真正改动本机文件。只有会改的才值得在更新前做备份，
    /// 否则就是白拷一份（GUI 程序动辄几百 MB）。当前所有适配器都只是提示用户手动更新。
    fn update_mutates_files(&self) -> bool { false }
}

pub fn all_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(adapters::ClaudeAdapter),
        Box::new(adapters::CodexAdapter),
        Box::new(adapters::HermesAdapter),
        Box::new(adapters::OpenClawAdapter),
        Box::new(adapters::WorkBuddyAdapter),
        Box::new(adapters::DeepSeekAdapter),
    ]
}

pub fn adapter_for(id: &str) -> Option<Box<dyn AgentAdapter>> {
    all_adapters().into_iter().find(|adapter| adapter.id() == id)
}

pub fn not_installed(id: &str, name: &str, publisher: &str, url: &str, detail: &str) -> AgentInstance {
    let adapter = adapter_for(id);
    let install_method = adapter.as_ref().map(|a| a.install_method()).unwrap_or(InstallMethod::OpenBrowser { url: url.to_string() });
    let install_url = match &install_method {
        InstallMethod::Winget { .. } => url.to_string(),
        InstallMethod::DirectDownload { url: u } => u.clone(),
        InstallMethod::OpenBrowser { url: u } => u.clone(),
    };
    AgentInstance {
        id: id.to_string(),
        name: name.to_string(),
        publisher: publisher.to_string(),
        installed: false,
        install_path: None,
        executable_path: None,
        version: None,
        latest_version: None,
        status: AgentStatus::NotInstalled,
        running: false,
        update_mode: if id == "workbuddy" || id == "deepseek" { UpdateMode::ManualAction } else { UpdateMode::NativeUpdater },
        official_url: url.to_string(),
        install_url,
        install_method,
        detail: detail.to_string(),
        last_checked: None,
    }
}

#[cfg(test)]
mod tests {
    use super::all_adapters;

    #[test]
    fn no_adapter_mutates_files_on_update_yet() {
        // 目前所有适配器的 update() 都只是提示用户手动更新，所以都不该声称会改本机文件
        //（否则每次点更新都会先做一次毫无意义的备份）。将来某个适配器真的会改文件时，
        // 这个断言会失败，提醒把它的 update_mutates_files() 改成 true。
        for adapter in all_adapters() {
            assert!(!adapter.update_mutates_files(), "{} 声称会改动本机文件，请确认更新前备份确实生效", adapter.id());
        }
    }
}
