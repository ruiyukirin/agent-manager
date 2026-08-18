use std::{env, fs, path::{Path, PathBuf}, process::{Command, Stdio}, time::{Duration, Instant}};

pub fn program_files() -> PathBuf { env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Program Files")) }
pub fn local_app_data() -> PathBuf { env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\AppData\\Local")) }
pub fn app_data() -> PathBuf { env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\AppData\\Roaming")) }
pub fn temp_dir() -> PathBuf { env::temp_dir() }
pub fn home_dir() -> Option<PathBuf> { env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| env::var_os("HOME").map(PathBuf::from)) }

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

fn parse_reg_query_value(output: &str, value_name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let name = parts.next()?;
        if !name.eq_ignore_ascii_case(value_name) {
            return None;
        }
        let _kind = parts.next()?;
        let value = parts.collect::<Vec<_>>().join(" ");
        if value.is_empty() { None } else { Some(value) }
    })
}

pub fn read_registry_display_version(key_name: &str) -> Option<String> {
    let output = Command::new("reg.exe").args(["query", &format!("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", key_name), "/v", "DisplayVersion"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_reg_query_value(text.as_ref(), "DisplayVersion")
}

pub fn read_registry_install_location(key_name: &str) -> Option<PathBuf> {
    let output = Command::new("reg.exe").args(["query", &format!("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", key_name), "/v", "InstallLocation"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_reg_query_value(text.as_ref(), "InstallLocation").map(PathBuf::from)
}

pub fn is_process_running(name: &str) -> bool {
    let output = Command::new("tasklist.exe").args(["/FI", &format!("IMAGENAME eq {}", name), "/NH"]).output();
    output.ok().map(|out| String::from_utf8_lossy(&out.stdout).lines().any(|line| line.to_ascii_lowercase().contains(&name.to_ascii_lowercase()))).unwrap_or(false)
}

pub fn launch(path: &Path) -> Result<(), String> {
    Command::new(path).spawn().map(|_| ()).map_err(|error| format!("无法启动 {}: {}", path.display(), error))
}

pub fn command_capture(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command).args(args).output().ok().and_then(|out| {
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if stdout.is_empty() && stderr.is_empty() { None } else { Some(format!("{stdout}\n{stderr}")) }
    })
}

pub fn command_capture_stdout(command: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(command).args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

pub fn command_capture_stdout_timeout(command: &str, args: &[&str], timeout: Duration) -> Option<String> {
    use std::io::Read;
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buffer = String::new();
        let _ = std::io::BufReader::new(stdout).read_to_string(&mut buffer);
        buffer
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return None;
            }
        }
    };

    let text = reader.join().ok()?;
    if !status.success() { return None; }
    let text = text.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

pub fn winget_latest_version(package_id: &str) -> Option<String> {
    let output = command_capture_stdout_timeout("winget.exe", &["show", "--id", package_id, "--accept-source-agreements"], Duration::from_secs(45))?;
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(ver) = trimmed.strip_prefix("Version:") {
            let ver = ver.trim();
            if !ver.is_empty() { return Some(ver.to_string()); }
        }
    }
    None
}

pub fn github_latest_version(owner: &str, repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let output = command_capture_stdout("curl.exe", &["-sL", "--fail", "--connect-timeout", "15", "--max-time", "30", &url])?;
    let value: serde_json::Value = serde_json::from_str(&output).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    let ver = tag.strip_prefix('v').unwrap_or(tag).to_string();
    Some(ver)
}

pub fn npm_latest_version(package_name: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{package_name}/latest");
    let output = command_capture_stdout("curl.exe", &["-sL", "--fail", "--connect-timeout", "15", "--max-time", "30", &url])?;
    let value: serde_json::Value = serde_json::from_str(&output).ok()?;
    value.get("version")?.as_str().map(|s| s.to_string())
}

pub fn curl_follow_redirect(url: &str) -> Option<String> {
    command_capture_stdout("curl.exe", &["-sL", "--fail", "--connect-timeout", "15", "--max-time", "30", "-o", "NUL", "-w", "%{url_effective}", url])
}

pub fn parse_version_from_filename(filename: &str) -> Option<String> {
    let parts: Vec<&str> = filename.split(&['_', '-', ' '][..]).collect();
    for part in &parts {
        let segments: Vec<&str> = part.split('.').collect();
        if segments.len() >= 3 && segments.iter().all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())) {
            return Some(part.to_string());
        }
    }
    None
}

pub fn is_update_available(installed: &str, latest: &str) -> bool {
    let installed_core = installed.split(['-', '+']).next().unwrap_or(installed).trim();
    let latest_core = latest.split(['-', '+']).next().unwrap_or(latest).trim();

    let installed_segments: Vec<&str> = installed_core.split('.').collect();
    let latest_segments: Vec<&str> = latest_core.split('.').collect();
    let max_len = installed_segments.len().max(latest_segments.len());
    for i in 0..max_len {
        let a = installed_segments.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let b = latest_segments.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        if a < b { return true; }
        if a > b { return false; }
    }

    // Same core version: an installed pre-release is older than a stable release.
    installed.contains('-') && !latest.contains('-')
}

pub fn official_url_for(id: &str) -> &'static str {
    match id { "workbuddy" => "https://workbuddy.ai/", "marvis" => "https://marvis.qq.com/download/exe", "claude" => "https://www.anthropic.com/claude-code",
    "codex" => "https://apps.microsoft.com/", "hermes" => "https://github.com/NousResearch/hermes-agent", "openclaw" => "https://openclaw.ai", _ => "https://example.invalid/" }
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

    #[test]
    fn parses_version_from_filename() {
        assert_eq!(super::parse_version_from_filename("marvis_4100100002_1.0.0.44_x64_6373.exe"), Some("1.0.0.44".into()));
        assert_eq!(super::parse_version_from_filename("Claude-1.30096.1-Setup.exe"), Some("1.30096.1".into()));
    }

    #[test]
    fn compares_versions() {
        assert!(super::is_update_available("1.0.0", "1.0.1"));
        assert!(!super::is_update_available("1.0.1", "1.0.0"));
        assert!(!super::is_update_available("1.0.0", "1.0.0"));
        assert!(super::is_update_available("0.146.0", "0.146.1"));
    }

    #[test]
    fn compares_prerelease_versions() {
        assert!(super::is_update_available("1.2.3-rc.1", "1.2.3"));
        assert!(!super::is_update_available("1.2.3", "1.2.3-rc.1"));
        assert!(super::is_update_available("1.9.0", "1.10.0"));
        assert!(!super::is_update_available("1.10.0", "1.9.0"));
    }

    #[test]
    fn parses_registry_display_version_value() {
        let output = "\n    DisplayVersion    REG_SZ    1.2.3\n";
        assert_eq!(super::parse_reg_query_value(output, "DisplayVersion"), Some("1.2.3".into()));
    }

    #[test]
    fn parses_registry_install_location_with_spaces() {
        let output = "\n    InstallLocation    REG_SZ    C:\\Program Files\\Tencent\\Marvis\n";
        assert_eq!(super::parse_reg_query_value(output, "InstallLocation"), Some("C:\\Program Files\\Tencent\\Marvis".into()));
    }

    #[test]
    fn parses_registry_expand_sz_value() {
        let output = "\n    InstallLocation    REG_EXPAND_SZ    %ProgramFiles%\\Tencent\\Marvis\n";
        assert_eq!(super::parse_reg_query_value(output, "InstallLocation"), Some("%ProgramFiles%\\Tencent\\Marvis".into()));
    }

    #[test]
    fn does_not_match_registry_value_name_prefix() {
        let output = "\n    DisplayVersionExtra    REG_SZ    1.2.3\n";
        assert_eq!(super::parse_reg_query_value(output, "DisplayVersion"), None);
    }

    #[test]
    fn captures_stdout_within_timeout() {
        let output = super::command_capture_stdout_timeout(
            "cmd.exe",
            &["/C", "echo", "hello-timeout"],
            std::time::Duration::from_secs(5),
        );
        assert_eq!(output.as_deref(), Some("hello-timeout"));
    }

    #[test]
    fn returns_none_when_command_times_out() {
        let output = super::command_capture_stdout_timeout(
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 5"],
            std::time::Duration::from_millis(300),
        );
        assert_eq!(output, None);
    }
}
pub fn winget_install(package_id: &str) -> Result<(), String> {
    let status = Command::new("winget.exe")
        .args(["install", "--id", package_id, "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity"])
        .status().map_err(|e| format!("无法启动 winget: {e}"))?;
    if status.success() { Ok(()) }
    else { Err(format!("winget 安装进程退出 (exit: {:?})", status.code())) }
}
