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

/// 在卸载表输出里按「显示名包含」查找条目，返回（DisplayVersion、InstallLocation）。
/// 有些安装项（如 DeepSeek Harness）的键名是随机 GUID，无法按名字直接查询，只能遍历。
pub fn parse_uninstall_entry(output: &str, name_contains: &str) -> Option<(Option<String>, Option<PathBuf>)> {
    let needle = name_contains.to_ascii_lowercase();
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in output.lines() {
        if line.trim_start().to_ascii_uppercase().starts_with("HKEY_") && !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.is_empty() { blocks.push(current); }

    blocks.into_iter().find_map(|block| {
        let display = parse_reg_query_value(&block, "DisplayName")?;
        if !display.to_ascii_lowercase().contains(&needle) { return None; }
        let version = parse_reg_query_value(&block, "DisplayVersion");
        let location = parse_reg_query_value(&block, "InstallLocation").map(PathBuf::from);
        Some((version, location))
    })
}

/// 遍历用户级 / 机器级卸载表，返回指定程序的最新可见版本与安装位置。
pub fn uninstall_entry(name_contains: &str) -> Option<(Option<String>, Option<PathBuf>)> {
    const ROOTS: [&str; 3] = [
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "HKLM\\Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ];
    ROOTS.iter().find_map(|root| {
        let output = Command::new("reg.exe").args(["query", root, "/s"]).output().ok()?;
        parse_uninstall_entry(String::from_utf8_lossy(&output.stdout).as_ref(), name_contains)
    })
}

/// 从 electron-updater 的 feed（generic provider 的 yml）里取版本号。
pub fn parse_feed_version(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("version:")?.trim().trim_matches('\'').trim();
        if value.is_empty() { None } else { Some(value.to_string()) }
    })
}

/// 从 electron-updater 的 feed 里取安装包直链。
pub fn parse_feed_installer(text: &str) -> Option<String> {
    text.lines().map(str::trim).find(|line| line.starts_with("http") && line.ends_with(".exe")).map(ToOwned::to_owned)
}

/// 从任意文本里抽出第一个形如 x.y.z 的版本号（可带 -rc.1 这类后缀）。
/// 取不到、或者拿到空串，一律返回 None —— 不允许 Option<String> 里出现 Some("") 这种非法状态。
pub fn normalize_version(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let starts_number = chars[i].is_ascii_digit()
            && (i == 0 || !(chars[i - 1].is_ascii_digit() || chars[i - 1] == '.'));
        if !starts_number { i += 1; continue; }

        let start = i;
        let mut j = i;
        while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') { j += 1; }
        let mut core_end = j;
        while core_end > start && chars[core_end - 1] == '.' { core_end -= 1; }
        let core: String = chars[start..core_end].iter().collect();

        if core.matches('.').count() >= 2 && !core.contains("..") {
            let mut end = core_end;
            if core_end < chars.len() && (chars[core_end] == '-' || chars[core_end] == '+') {
                let mut k = core_end + 1;
                while k < chars.len() && (chars[k].is_ascii_alphanumeric() || chars[k] == '.' || chars[k] == '-') { k += 1; }
                while k > core_end + 1 && (chars[k - 1] == '.' || chars[k - 1] == '-') { k -= 1; }
                if k > core_end + 1 { end = k; }
            }
            return Some(chars[start..end].iter().collect());
        }
        i = j.max(i + 1);
    }
    None
}

/// 抽出形如 2026.9.14 的日期版本号。
/// Hermes 这类 Agent 的云端发布用的是日期版本（GitHub 标签 v2026.9.24），
/// 而程序自报的第一个版本号是内部 semver（v0.21.3）—— 两者跨体系，不可直接比大小。
pub fn normalize_date_version(text: &str) -> Option<String> {
    let mut rest = text;
    loop {
        let found = normalize_version(rest)?;
        let is_date = found.split('.').next().map(|head| head.len() == 4 && head.starts_with("20")).unwrap_or(false);
        if is_date { return Some(found); }
        let index = rest.find(&found)?;
        rest = &rest[index + found.len()..];
    }
}

/// 一条统一的版本探测链，按可靠性从高到低取第一个能解析出版本号的来源：
/// ① 程序自报（`--version`）② 厂商声明的版本（卸载表）③ 可执行文件内嵌版本 ④ 路径推断。
/// 每一级的原始输出都经过 normalize_version 归一化。
pub struct VersionProbe<'a> {
    /// 可以安全执行 `--version` 的程序（终端类工具）。GUI 程序不要填，避免误弹出窗口。
    pub cli: Option<(&'a Path, &'a [&'a str])>,
    /// 厂商声明的版本，与云端发布版本同一体系（如卸载表的 DisplayVersion）。
    pub declared: &'a [Option<String>],
    /// 可执行文件内嵌版本（PE 的 ProductVersion）。
    pub pe: Option<&'a Path>,
    /// 从路径推断，如 `OpenAI.Codex_26.803.10989.0_x64...` 这类目录名。
    pub path_hint: Option<&'a Path>,
}

pub fn probe_version(probe: &VersionProbe) -> Option<String> {
    let from_cli = probe.cli.and_then(|(exe, args)| command_capture(&exe.to_string_lossy(), args));
    let from_pe = probe.pe.and_then(read_pe_version);
    let from_path = probe.path_hint.and_then(parse_version_from_path);
    std::iter::once(from_cli)
        .chain(probe.declared.iter().cloned())
        .chain([from_pe, from_path])
        .flatten()
        .find_map(|raw| normalize_version(&raw))
}

/// 从可执行文件向上找名为 name 的祖先目录（大小写不敏感），找不到就退回文件所在目录。
/// 用来避免「父目录取几级」这种写死的算术 —— 不同 Agent 的目录层级并不一样。
pub fn install_root_for(exe: &Path, name: &str) -> PathBuf {
    let needle = name.to_ascii_lowercase();
    let mut current = exe.parent();
    while let Some(dir) = current {
        let matched = dir.file_name().and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case(&needle)).unwrap_or(false);
        if matched { return dir.to_path_buf(); }
        if dir.parent().is_none() { break; }
        current = dir.parent();
    }
    exe.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(exe))
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
    match id { "workbuddy" => "https://workbuddy.ai/", "deepseek" => "https://github.com/deepseek-ai/deepseek-harness", "claude" => "https://www.anthropic.com/claude-code",
    "codex" => "https://apps.microsoft.com/", "hermes" => "https://github.com/NousResearch/hermes-agent", "openclaw" => "https://openclaw.ai", _ => "https://example.invalid/" }
}

#[cfg(test)]
mod tests {
    use super::parse_version_from_path;
    use std::path::Path;

    #[test]
    fn parses_bootstrap_version_from_under_score_name() {
        let path = Path::new("C:\\Temp\\installer_4100100002_1.0.0.44_x64_6373.exe");
        assert_eq!(parse_version_from_path(path).as_deref(), Some("1.0.0.44"));
    }

    #[test]
    fn ignores_non_version_segments() {
        let path = Path::new("C:\\Temp\\agent_1.2.exe");
        assert_eq!(parse_version_from_path(path), None);
    }

    #[test]
    fn parses_version_from_filename() {
        assert_eq!(super::parse_version_from_filename("installer_4100100002_1.0.0.44_x64_6373.exe"), Some("1.0.0.44".into()));
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
        let output = "\n    InstallLocation    REG_SZ    C:\\Program Files\\Tencent\\WorkBuddy\n";
        assert_eq!(super::parse_reg_query_value(output, "InstallLocation"), Some("C:\\Program Files\\Tencent\\WorkBuddy".into()));
    }

    #[test]
    fn parses_registry_expand_sz_value() {
        let output = "\n    InstallLocation    REG_EXPAND_SZ    %ProgramFiles%\\Tencent\\WorkBuddy\n";
        assert_eq!(super::parse_reg_query_value(output, "InstallLocation"), Some("%ProgramFiles%\\Tencent\\WorkBuddy".into()));
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

    #[test]
    fn finds_uninstall_entry_by_display_name() {
        // 真实 reg.exe query /s 输出：注意键名是随机 GUID，且有的条目 DisplayVersion 排在 DisplayName 之前。
        let output = concat!(
            "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\300f80e0-781e-56db-ae9d-9d0190486ca9\n",
            "    DisplayVersion    REG_SZ    6.9.3\n",
            "    DisplayName    REG_SZ    阿里云盘\n\n",
            "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\1bf39983-50d0-5fe0-9ef4-cece76f67c5e\n",
            "    DisplayName    REG_SZ    DeepSeek Harness 0.2.0-rc.2\n",
            "    UninstallString    REG_SZ    \"C:\\Programs\\DeepSeek Harness\\Uninstall DeepSeek Harness.exe\" /currentuser\n",
            "    DisplayVersion    REG_SZ    0.2.0-rc.2\n",
            "    InstallLocation    REG_SZ    C:\\Programs\\DeepSeek Harness\n"
        );
        let (version, location) = super::parse_uninstall_entry(output, "DeepSeek Harness").expect("应能按显示名找到条目");
        assert_eq!(version.as_deref(), Some("0.2.0-rc.2"));
        assert_eq!(location, Some(std::path::PathBuf::from("C:\\Programs\\DeepSeek Harness")));
        assert!(super::parse_uninstall_entry(output, "不存在程序").is_none());
    }

    #[test]
    fn parses_electron_update_feed() {
        // 真实 nightly.yml 结构（folded scalar 换行缩进）。
        let feed = concat!(
            "version: 0.2.0-rc.2\n",
            "files:\n",
            "  - url: >-\n",
            "      https://download.deepseek.com/dsh-desk/bin/win-x64/deepseek-harness-0.2.0-rc.2-win-x64.exe\n",
            "    sha512: >-\n",
            "      raIlxMQd9ESXgktmViW7QwVLcjBR5JIsrNOu+SpelY8kskdSr2H51/f+ey1EqFI/eIIrKQCuRANMeb5SptRZcg==\n",
            "    size: 289313640\n",
            "path: >-\n",
            "  https://download.deepseek.com/dsh-desk/bin/win-x64/deepseek-harness-0.2.0-rc.2-win-x64.exe\n",
            "releaseDate: '2026-09-29T10:35:27.666Z'\n"
        );
        assert_eq!(super::parse_feed_version(feed).as_deref(), Some("0.2.0-rc.2"));
        assert_eq!(
            super::parse_feed_installer(feed).as_deref(),
            Some("https://download.deepseek.com/dsh-desk/bin/win-x64/deepseek-harness-0.2.0-rc.2-win-x64.exe")
        );
        assert_eq!(super::parse_feed_version("files: []\n"), None);
        assert_eq!(super::parse_feed_installer("version: 1.0.0\n"), None);
    }

    #[test]
    fn normalizes_real_version_outputs() {
        // 以下三条是本机真实输出，作为回归用例固化。
        assert_eq!(super::normalize_version("codex-cli 0.155.0-alpha.9.2").as_deref(), Some("0.155.0-alpha.9.2"));
        assert_eq!(
            super::normalize_version("Hermes Agent v0.21.3 (2026.9.14) · upstream c62bd9f2 | Install directory: C:\\x\\hermes-agent").as_deref(),
            Some("0.21.3")
        );
        assert_eq!(super::normalize_version("5.6.2.0").as_deref(), Some("5.6.2.0"));
        assert_eq!(super::normalize_version("1.2.3-rc.1").as_deref(), Some("1.2.3-rc.1"));
        // 空串、抽不到、只有两段、只有一段，都必须返回 None（不允许 Some("")）。
        assert_eq!(super::normalize_version(""), None);
        assert_eq!(super::normalize_version("no version here"), None);
        assert_eq!(super::normalize_version("1.2"), None);
        assert_eq!(super::normalize_version("C:\\Temp\\agent_1.2.exe"), None);
        assert_eq!(super::normalize_version("releaseDate: 2026-09-29T10:35:27.666Z"), None);
    }

    #[test]
    fn probes_version_in_reliability_order() {
        let exe = Path::new("cmd.exe");
        let args: &[&str] = &["/C", "echo", "codex-cli 0.155.0-alpha.9.2"];
        let declared: &[Option<String>] = &[Some("0.146.1".into())];
        let none: &[Option<String>] = &[];

        // ① 程序自报优先于卸载表声明（Codex 本机就有两套安装：自更新版 0.155，winget 版 0.146.1）
        let probe = super::VersionProbe { cli: Some((exe, args)), declared, pe: None, path_hint: None };
        assert_eq!(super::probe_version(&probe).as_deref(), Some("0.155.0-alpha.9.2"));

        // ② 不能执行命令行时，用卸载表声明的版本（WorkBuddy 是 GUI 程序，不能瞎执行）
        let probe = super::VersionProbe { cli: None, declared, pe: None, path_hint: None };
        assert_eq!(super::probe_version(&probe).as_deref(), Some("0.146.1"));

        // ③ 声明里混进空串要跳过，不能当成拿到版本
        let with_empty: &[Option<String>] = &[Some(String::new()), Some("0.2.0-rc.2".into())];
        let probe = super::VersionProbe { cli: None, declared: with_empty, pe: None, path_hint: None };
        assert_eq!(super::probe_version(&probe).as_deref(), Some("0.2.0-rc.2"));

        // ④ 全都拿不到就是 None
        let probe = super::VersionProbe { cli: None, declared: none, pe: None, path_hint: None };
        assert_eq!(super::probe_version(&probe), None);
    }

    #[test]
    fn finds_install_root_by_directory_name() {
        assert_eq!(
            super::install_root_for(Path::new("C:\\Users\\me\\AppData\\Local\\Programs\\WorkBuddy\\WorkBuddy.exe"), "WorkBuddy"),
            std::path::PathBuf::from("C:\\Users\\me\\AppData\\Local\\Programs\\WorkBuddy")
        );
        assert_eq!(
            super::install_root_for(Path::new("C:\\x\\hermes-agent\\venv\\Scripts\\hermes.exe"), "hermes-agent"),
            std::path::PathBuf::from("C:\\x\\hermes-agent")
        );
        // 找不到同名祖先时退回文件所在目录
        assert_eq!(
            super::install_root_for(Path::new("C:\\x\\bin\\tool.exe"), "not-there"),
            std::path::PathBuf::from("C:\\x\\bin")
        );
    }

    #[test]
    fn picks_date_version_for_date_based_agents() {
        // Hermes 的真实输出：内部 semver 在前，云端可比的日期版本在后。
        let line = "Hermes Agent v0.21.3 (2026.9.14) · upstream c62bd9f2 | Install directory: C:\\x\\hermes-agent";
        assert_eq!(super::normalize_version(line).as_deref(), Some("0.21.3"));
        assert_eq!(super::normalize_date_version(line).as_deref(), Some("2026.9.14"));
        // 云端标签就是日期体系，两者可以正常比较
        assert!(super::is_update_available("2026.9.14", "2026.9.24"));
        assert!(!super::is_update_available("2026.9.24", "2026.9.24"));
        // 没有日期版本时不要硬造
        assert_eq!(super::normalize_date_version("codex-cli 0.155.0-alpha.9.2"), None);
    }
}
pub fn winget_install(package_id: &str) -> Result<(), String> {
    let status = Command::new("winget.exe")
        .args(["install", "--id", package_id, "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity"])
        .status().map_err(|e| format!("无法启动 winget: {e}"))?;
    if status.success() { Ok(()) }
    else { Err(format!("winget 安装进程退出 (exit: {:?})", status.code())) }
}
