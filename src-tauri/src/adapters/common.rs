use std::{env, fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};
use sha2::{Digest, Sha256};

pub fn program_files() -> PathBuf { env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Program Files")) }
pub fn local_app_data() -> PathBuf { env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\AppData\\Local")) }
pub fn app_data() -> PathBuf { env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\AppData\\Roaming")) }
pub fn temp_dir() -> PathBuf { env::temp_dir() }

pub fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> { paths.iter().find(|path| path.exists()).cloned() }
pub fn first_file(paths: &[PathBuf]) -> Option<PathBuf> { paths.iter().find(|path| path.is_file()).cloned() }
pub fn find_child(root: &Path, name: &str) -> Option<PathBuf> { fs::read_dir(root).ok()?.flatten().map(|entry| entry.path()).find(|path| path.file_name().and_then(|value| value.to_str()) == Some(name)) }
pub fn find_child_prefix(root: &Path, prefix: &str) -> Option<PathBuf> { fs::read_dir(root).ok()?.flatten().map(|entry| entry.path()).find(|path| path.file_name().and_then(|value| value.to_str()).map(|value| value.starts_with(prefix)).unwrap_or(false)) }
pub fn canonical(path: &Path) -> Option<PathBuf> { fs::canonicalize(path).ok() }

pub fn path_candidates(base: &[PathBuf], names: &[&str]) -> Vec<PathBuf> { base.iter().flat_map(|root| names.iter().map(move |name| root.join(name))).collect() }

pub fn read_text(path: &Path) -> Option<String> { fs::read_to_string(path).ok() }
pub fn parse_setup_version(path: &Path) -> Option<String> { let text = read_text(path)?; let value: serde_json::Value = serde_json::from_str(&text).ok()?; value.get("config")?.get("version")?.as_str().map(ToOwned::to_owned) }

pub fn parse_version_from_path(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    let candidate = text.rsplit(['\\', '/']).next()?;
    let version = candidate.split('_').find_map(|part| if part.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) && part.matches('.').count() >= 2 { Some(part) } else { None });
    version.map(|value| value.to_string())
}

pub fn read_pe_version(path: &Path) -> Option<String> {
    let escaped = path.to_string_lossy().replace("'", "''");
    let script = format!("(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion", escaped);
    let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", &script]).output().ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

pub fn version_from_file(path: &Path) -> Option<String> {
    if path.extension().and_then(|v| v.to_str()) == Some("json") { return parse_setup_version(path); }
    read_pe_version(path).or_else(|| parse_version_from_path(path))
}

pub fn read_registry_display_version(key_name: &str) -> Option<String> {
    let output = Command::new("reg.exe").args(["query", &format!("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", key_name), "/v", "DisplayVersion"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| { let mut parts = line.split_whitespace(); while let Some(part) = parts.next() { if part.eq_ignore_ascii_case("DisplayVersion") { return parts.next().map(ToOwned::to_owned); } } None })
}

pub fn read_registry_install_location(key_name: &str) -> Option<PathBuf> {
    let output = Command::new("reg.exe").args(["query", &format!("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", key_name), "/v", "InstallLocation"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| { let mut parts = line.split_whitespace(); while let Some(part) = parts.next() { if part.eq_ignore_ascii_case("InstallLocation") { return parts.next().map(PathBuf::from); } } None })
}

pub fn is_process_running(name: &str) -> bool {
    let output = Command::new("tasklist.exe").args(["/FI", &format!("IMAGENAME eq {}", name), "/NH"]).output();
    output.ok().map(|out| String::from_utf8_lossy(&out.stdout).lines().any(|line| line.to_ascii_lowercase().contains(&name.to_ascii_lowercase()))).unwrap_or(false)
}

pub fn launch(path: &Path) -> Result<(), String> {
    Command::new(path).spawn().map(|_| ()).map_err(|error| format!("无法启动 {}: {}", path.display(), error))
}

pub fn command_capture(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command).args(args).output().ok().map(|out| format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
}

pub fn official_url_for(id: &str) -> &'static str {
    match id { "workbuddy" => "https://workbuddy.ai/", "marvis" => "https://marvis.qq.com/download/exe", "claude" => "https://www.anthropic.com/claude-code",
    "codex" => "https://apps.microsoft.com/", "hermes" => "https://github.com/NousResearch/hermes-agent", "openclaw" => "https://openclaw.ai", _ => "https://example.invalid/" }
}

pub fn sha256(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(format!("{:x}", digest))
}

pub fn download_to_temp(url: &str, prefix: &str) -> Result<PathBuf, String> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis();
    let downloads = app_data().join("AgentManager\\downloads"); let _ = fs::create_dir_all(&downloads); let output = downloads.join(format!("{}_{}.exe", prefix, stamp));
    let result = Command::new("curl.exe").args(["-fL", "--max-time", "45", url, "-o"]).arg(&output).output().map_err(|e| format!("无法启动 curl: {}", e))?;
    if !result.status.success() { return Err(format!("下载官方安装包失败：{}", String::from_utf8_lossy(&result.stderr).trim())); }
    let bytes = fs::read(&output).map_err(|e| e.to_string())?;
    if bytes.len() < 2 || &bytes[..2] != b"MZ" { return Err("官方地址没有返回 Windows PE 安装包".to_string()); }
    if bytes.len() > 256 * 1024 * 1024 { return Err("官方安装包超过 256 MB 安全上限".to_string()); }
    Ok(output)
}





#[cfg(test)]
mod tests {
    use super::parse_version_from_path;
    use std::path::Path;

    #[test]
    fn parses_bootstrap_version_from_under_score_name() {
        let path = Path::new("C:\\Temp\\marvis_4100100002_1.0.0.44_x64_6373.exe");
        assert_eq!(parse_version_from_path(path).as_deref(), Some("1.0.0.44"));
    }

    #[test]
    fn ignores_non_version_segments() {
        let path = Path::new("C:\\Temp\\agent_1.2.exe");
        assert_eq!(parse_version_from_path(path), None);
    }
}

