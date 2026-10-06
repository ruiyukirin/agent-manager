use super::common::*;
use crate::adapter::{not_installed, AgentAdapter};
use crate::model::{AgentInstance, AgentStatus, InstallMethod, UpdateMode, UpdateResult};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 官方更新源：取自安装目录 resources/app-update.yml（provider: generic / channel: nightly）。
const FEED_URL: &str = "https://download.deepseek.com/dsh-desk/feeds/win-x64/nightly.yml";
/// feed 不可达时退回的安装包地址。版本升级后由 feed 里的直链覆盖，不需要跟着改。
const FALLBACK_INSTALLER: &str = "https://download.deepseek.com/dsh-desk/bin/win-x64/deepseek-harness-0.2.0-rc.2-win-x64.exe";
const DISPLAY_NAME: &str = "DeepSeek Harness";
const EXECUTABLE: &str = "DeepSeek Harness.exe";
const UNINSTALLER: &str = "Uninstall DeepSeek Harness.exe";

/// 官方 feed 的进程内缓存：扫描路径不该反复发网络请求，同时让安装时能用到最近一次查到的直链。
static FEED_CACHE: OnceLock<Option<String>> = OnceLock::new();

fn fetch_feed() -> Option<String> {
    command_capture_stdout("curl.exe", &["-sL", "--fail", "--connect-timeout", "15", "--max-time", "30", FEED_URL])
}

fn cached_feed() -> Option<&'static str> {
    FEED_CACHE.get_or_init(fetch_feed).as_deref()
}

pub struct DeepSeekAdapter;

impl DeepSeekAdapter {
    fn default_root() -> PathBuf { local_app_data().join("Programs\\DeepSeek Harness") }

    /// 安装位置与当前版本。卸载键名是随机 GUID，只能遍历卸载表；
    /// 另外主程序自身的内嵌版本是 Electron 的 0.2.0.0（会把应用版本吃掉），所以 PE 只认卸载程序。
    fn installed(&self) -> (PathBuf, Option<String>) {
        let entry = uninstall_entry(DISPLAY_NAME);
        let root = entry.as_ref().and_then(|(_, location)| location.clone()).unwrap_or_else(Self::default_root);
        let declared = [entry.as_ref().and_then(|(version, _)| version.clone())];
        let version = probe_version(&VersionProbe {
            cli: None,
            declared: &declared,
            pe: Some(&root.join(UNINSTALLER)),
            path_hint: None,
        });
        (root, version)
    }
}

impl AgentAdapter for DeepSeekAdapter {
    fn id(&self) -> &'static str { "deepseek" }
    fn display_name(&self) -> &'static str { DISPLAY_NAME }
    fn publisher(&self) -> &'static str { "DeepSeek" }
    fn official_url(&self) -> &'static str { official_url_for("deepseek") }

    fn discover(&self) -> Result<AgentInstance, String> {
        let (root, version) = self.installed();
        let executable = first_file(&[root.join(EXECUTABLE)]);
        if !root.exists() && executable.is_none() {
            return Ok(not_installed(self.id(), self.display_name(), self.publisher(), self.official_url(), "未发现 DeepSeek Harness 安装"));
        }
        Ok(AgentInstance {
            id: self.id().into(), name: self.display_name().into(), publisher: self.publisher().into(), installed: true,
            install_path: Some(root.to_string_lossy().into_owned()),
            executable_path: executable.as_ref().map(|p| p.to_string_lossy().into_owned()),
            version: Some(crate::model::VersionSnapshot { product: version.clone(), component: None, bootstrap: None, channel: Some("nightly".into()), pe: version }),
            latest_version: None, status: AgentStatus::Checking, running: is_process_running(EXECUTABLE),
            update_mode: UpdateMode::ManualAction,
            official_url: self.official_url().into(), install_url: FALLBACK_INSTALLER.into(),
            install_method: InstallMethod::DirectDownload { url: FALLBACK_INSTALLER.into() },
            detail: "Electron 桌面版（内置 dsh 运行时 + 插件）".into(), last_checked: None,
        })
    }

    fn check_latest(&self, agent: &mut AgentInstance) -> Result<(), String> {
        // 用户主动点的检查，每次重新取；顺手写进缓存给安装路径复用。
        let feed = fetch_feed();
        if let Some(text) = feed.as_deref() { let _ = FEED_CACHE.set(Some(text.to_string())); }
        // feed 里的直链带当前版本号，取到就用它覆盖，避免安装包地址随版本升级失效。
        if let Some(url) = feed.as_deref().and_then(parse_feed_installer) {
            agent.install_url = url.clone();
            agent.install_method = InstallMethod::DirectDownload { url };
        }
        match feed.as_deref().and_then(parse_feed_version) {
            Some(latest) => {
                agent.latest_version = Some(latest.clone());
                let current = agent.version.as_ref().and_then(|v| v.product.as_deref().or(v.pe.as_deref())).unwrap_or("");
                if current.is_empty() {
                    agent.status = AgentStatus::ManualAction;
                    agent.detail = format!("云端最新版本：{}", latest);
                } else if is_update_available(current, &latest) {
                    agent.status = AgentStatus::UpdateAvailable;
                    agent.detail = format!("{} → {} 可更新（nightly 通道）", current, latest);
                } else {
                    agent.status = AgentStatus::UpToDate;
                    agent.detail = "已是最新版本".into();
                }
            }
            None => {
                agent.latest_version = None;
                agent.status = AgentStatus::ManualAction;
                agent.detail = "无法读取官方更新源（nightly 通道），请使用应用内置更新".into();
            }
        }
        agent.last_checked = Some("刚刚".into());
        Ok(())
    }

    fn update(&self, _agent: &mut AgentInstance) -> Result<UpdateResult, String> {
        Ok(UpdateResult {
            success: false,
            message: "DeepSeek Harness 使用应用内置更新（nightly 通道）；也可通过官方入口重新下载安装".into(),
            mode: UpdateMode::ManualAction,
            official_url: Some(self.official_url().into()),
            needs_restart: false,
            previous_version: None,
            current_version: None,
        })
    }

    fn install(&self) -> Result<(), String> { Err("DeepSeek Harness 请使用官方安装包或应用内置更新".into()) }

    fn install_method(&self) -> InstallMethod {
        let url = cached_feed().and_then(parse_feed_installer).unwrap_or_else(|| FALLBACK_INSTALLER.to_string());
        InstallMethod::DirectDownload { url }
    }

    fn restart_if_was_running(&self, agent: &mut AgentInstance, was_running: bool) -> Result<(), String> {
        if was_running { if let Some(path) = agent.executable_path.as_deref() { return launch(Path::new(path)); } }
        Ok(())
    }
}
