// DSH Desktop Shell - M1
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub dsh_command_template: String,
    pub port: u16,
    pub auto_open_browser: bool,
    pub auto_start_on_launch: bool,
    #[serde(default)]
    pub auto_launch_on_boot: bool,
    pub minimize_to_tray: bool,
    pub global_shortcut: String,
    pub theme: String,
    pub log_retention_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dsh_command_template: "npx @deepseek-ai/dsh web".into(),
            port: 3080,
            auto_open_browser: true,
            auto_start_on_launch: false,
            auto_launch_on_boot: false,
            minimize_to_tray: true,
            global_shortcut: "Alt+Shift+D".into(),
            theme: "system".into(),
            log_retention_days: 7,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DshStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub state: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command_template: String,
    pub cwd: Option<String>,
    pub auto_open_url: Option<String>,
    pub icon: String,
    pub color: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStatus {
    pub running: bool,
    pub pid: Option<u32>,
}


pub struct AppState {
    pub child: Mutex<Option<Child>>,
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    pub log_file: PathBuf,
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
    pub plugins: Mutex<Vec<PluginConfig>>,
    pub plugin_processes: Mutex<HashMap<String, Child>>,
}

fn data_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("dsh-desktop")
}

fn settings_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("settings.json")
}

fn load_settings(data_dir: &PathBuf) -> Settings {
    let path = settings_path(data_dir);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn persist_settings(data_dir: &PathBuf, settings: &Settings) -> Result<(), String> {
    let path = settings_path(data_dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn set_auto_launch(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()
            .map_err(|e| format!("获取程序路径失败: {e}"))?;
        let exe_str = exe.to_string_lossy().to_string();

        if enabled {
            let value = format!("\"{exe_str}\"");
            let output = Command::new("reg")
                .arg("add")
                .arg("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run")
                .arg("/v")
                .arg("DSHDesktop")
                .arg("/t")
                .arg("REG_SZ")
                .arg("/d")
                .arg(&value)
                .arg("/f")
                .output()
                .map_err(|e| format!("写入开机自启动失败: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "写入开机自启动失败，状态码: {:?}",
                    output.status.code()
                ));
            }
        } else {
            let output = Command::new("reg")
                .arg("delete")
                .arg("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run")
                .arg("/v")
                .arg("DSHDesktop")
                .arg("/f")
                .output()
                .map_err(|e| format!("删除开机自启动失败: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "删除开机自启动失败，状态码: {:?}",
                    output.status.code()
                ));
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
    }

    Ok(())
}


fn plugins_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("plugins.json")
}

fn default_plugins() -> Vec<PluginConfig> {
    vec![PluginConfig {
        id: "dsh-web".into(),
        name: "DSH Web".into(),
        description: "启动 DeepSeek Harness 网页版".into(),
        command_template: "npx @deepseek-ai/dsh web".into(),
        cwd: None,
        auto_open_url: Some("http://127.0.0.1:{port}".into()),
        icon: "🌐".into(),
        color: "#4F46E5".into(),
        enabled: true,
    }]
}


fn load_plugins(data_dir: &PathBuf) -> Vec<PluginConfig> {
    let path = plugins_path(data_dir);
    if !path.exists() {
        let defaults = default_plugins();
        let _ = persist_plugins(data_dir, &defaults);
        return defaults;
    }

    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn persist_plugins(data_dir: &PathBuf, plugins: &[PluginConfig]) -> Result<(), String> {
    let path = plugins_path(data_dir);
    let json = serde_json::to_string_pretty(plugins).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn plugin_log_file(data_dir: &PathBuf, id: &str) -> PathBuf {
    let logs = data_dir.join("logs");
    let _ = fs::create_dir_all(&logs);
    logs.join(format!("plugin-{id}.log"))
}


fn current_log_file(data_dir: &PathBuf) -> PathBuf {
    let logs = data_dir.join("logs");
    let _ = fs::create_dir_all(&logs);
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    logs.join(format!("dsh-{secs}.log"))
}

fn cleanup_logs(data_dir: &PathBuf, retention_days: u32) {
    let logs_dir = data_dir.join("logs");
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days as u64 * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    if let Ok(entries) = fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}


fn open_in_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let start = SystemTime::now();
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        if start.elapsed().unwrap_or_default() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}


#[cfg(windows)]
fn get_child_process_ids(parent_pid: u32) -> Vec<u32> {
    let output = Command::new("wmic")
        .arg("process")
        .arg("where")
        .arg(format!("ParentProcessId={parent_pid}"))
        .arg("get")
        .arg("ProcessId")
        .output();

    let mut ids = Vec::new();
    if let Ok(output) = output {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if let Ok(pid) = trimmed.parse::<u32>() {
                    if pid != 0 {
                        ids.push(pid);
                    }
                }
            }
        }
    }
    ids
}

#[cfg(windows)]
fn kill_process_tree(pid: u32) {
    for child in get_child_process_ids(pid) {
        kill_process_tree(child);
    }
    let _ = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/F")
        .output();
}

#[cfg(windows)]
fn kill_processes_by_commandline(marker: &str) {
    let where_clause = format!("CommandLine like '%{}%'", marker.replace('\'', "''"));
    let output = Command::new("wmic")
        .arg("process")
        .arg("where")
        .arg(where_clause)
        .arg("get")
        .arg("ProcessId")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if pid != 0 {
                        let _ = Command::new("taskkill")
                            .arg("/PID")
                            .arg(pid.to_string())
                            .arg("/F")
                            .output();
                    }
                }
            }
        }
    }
}



fn read_and_log<R: Read + Send + 'static>(
    reader: R,
    log_path: PathBuf,
    is_stderr: bool,
    app: AppHandle,
) {
    let reader = BufReader::new(reader);
    let prefix = if is_stderr { "[stderr]" } else { "[stdout]" };
    for line in reader.lines() {
        if let Ok(line) = line {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let entry = format!("[{secs}] {prefix} {line}\n");
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = file.write_all(entry.as_bytes());
            }
            let _ = app.emit("dsh-log", entry.trim_end().to_string());
        }
    }
}

fn start_dsh_inner(app: &AppHandle, state: &AppState) -> Result<DshStatus, String> {
    let mut guard = state.child.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        if let Ok(None) = child.try_wait() {
            return Ok(DshStatus {
                running: true,
                pid: Some(child.id()),
                state: "running".into(),
                exit_code: None,
            });
        }
    }

    let settings = state.settings.lock().unwrap().clone();
    let command_line = settings
        .dsh_command_template
        .replace("{port}", &settings.port.to_string());

    let log_path = state.log_file.clone();
    let mut command = Command::new("cmd");
    command.arg("/C").arg(&command_line);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("启动 DSH 失败: {e}"))?;

    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let log = log_path.clone();
        std::thread::spawn(move || read_and_log(stdout, log, false, app));
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let log = log_path.clone();
        std::thread::spawn(move || read_and_log(stderr, log, true, app));
    }

    *guard = Some(child);

    if settings.auto_open_browser {
        let port = settings.port;
        let url = format!("http://127.0.0.1:{port}");
        std::thread::spawn(move || {
            if wait_for_port(port, Duration::from_secs(30)) {
                open_in_browser(&url);
            } else {
                eprintln!("等待 DSH Web 启动超时，未打开浏览器");
            }
        });
    }

    let _ = app.emit("dsh-status", "running");

    Ok(DshStatus {
        running: true,
        pid: Some(pid),
        state: "running".into(),
        exit_code: None,
    })
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    *state.settings.lock().unwrap() = settings.clone();
    persist_settings(&state.data_dir, &settings)?;
    set_auto_launch(settings.auto_launch_on_boot)
}

#[tauri::command]
fn start_dsh(app: AppHandle, state: State<'_, AppState>) -> Result<DshStatus, String> {
    start_dsh_inner(&app, state.inner())
}

fn stop_dsh_inner(state: &AppState) -> Result<(), String> {
    let mut guard = state.child.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        let pid = child.id();
        #[cfg(windows)]
        {
            kill_process_tree(pid);
            kill_processes_by_commandline("deepseek-ai");
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
        guard.take();
        Ok(())
    } else {
        Err("DSH 当前没有在运行".into())
    }
}


#[tauri::command]
fn stop_dsh(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.child.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        let pid = child.id();
        #[cfg(windows)]
        {
            kill_process_tree(pid);
            kill_processes_by_commandline("deepseek-ai");
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
        guard.take();
        Ok(())
    } else {
        Err("DSH 当前没有在运行".into())
    }
}

fn start_plugin_inner(app: &AppHandle, state: &AppState, id: &str) -> Result<PluginStatus, String> {
    let plugin = {
        let plugins = state.plugins.lock().unwrap();
        plugins
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| format!("插件不存在: {id}"))?
    };

    if !plugin.enabled {
        return Err("插件已禁用".into());
    }

    let mut processes = state.plugin_processes.lock().unwrap();
    if let Some(child) = processes.get_mut(id) {
        if let Ok(None) = child.try_wait() {
            return Ok(PluginStatus {
                running: true,
                pid: Some(child.id()),
            });
        }
    }

    let settings = state.settings.lock().unwrap().clone();
    let command_line = plugin
        .command_template
        .replace("{port}", &settings.port.to_string());

    let log_path = plugin_log_file(&state.data_dir, &plugin.id);
    let mut command = Command::new("cmd");
    command.arg("/C").arg(&command_line);
    if let Some(cwd) = &plugin.cwd {
        if !cwd.is_empty() {
            command.current_dir(cwd);
        }
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("启动插件失败: {e}"))?;

    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let log = log_path.clone();
        std::thread::spawn(move || read_and_log(stdout, log, false, app));
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let log = log_path.clone();
        std::thread::spawn(move || read_and_log(stderr, log, true, app));
    }

    processes.insert(id.to_string(), child);

    if let Some(url) = &plugin.auto_open_url {
        let url = url.replace("{port}", &settings.port.to_string());
        open_in_browser(&url);
    }

    let _ = app.emit("plugin-status", id.to_string());

    Ok(PluginStatus {
        running: true,
        pid: Some(pid),
    })
}

fn stop_plugin_inner(state: &AppState, id: &str) -> Result<(), String> {
    let plugin = {
        let plugins = state.plugins.lock().unwrap();
        plugins.iter().find(|p| p.id == id).cloned()
    };

    let mut processes = state.plugin_processes.lock().unwrap();
    if let Some(mut child) = processes.remove(id) {
        let pid = child.id();
        #[cfg(windows)]
        {
            kill_process_tree(pid);
            if let Some(plugin) = &plugin {
                if plugin.command_template.contains("deepseek-ai") {
                    kill_processes_by_commandline("deepseek-ai");
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
        Ok(())
    } else {
        Err("插件当前没有在运行".into())
    }
}

#[tauri::command]
fn get_plugins(state: State<'_, AppState>) -> Result<Vec<PluginConfig>, String> {
    let mut plugins = state.plugins.lock().unwrap();
    if plugins.is_empty() && !plugins_path(&state.data_dir).exists() {
        let defaults = default_plugins();
        persist_plugins(&state.data_dir, &defaults)?;
        *plugins = defaults;
    }
    Ok(plugins.clone())
}

#[tauri::command]
fn save_plugin(state: State<'_, AppState>, plugin: PluginConfig) -> Result<Vec<PluginConfig>, String> {
    if plugin.id.trim().is_empty() {
        return Err("插件 ID 不能为空".into());
    }

    let mut plugins = state.plugins.lock().unwrap();
    if let Some(existing) = plugins.iter_mut().find(|p| p.id == plugin.id) {
        *existing = plugin;
    } else {
        plugins.push(plugin);
    }

    let snapshot = plugins.clone();
    persist_plugins(&state.data_dir, &snapshot)?;
    Ok(snapshot)
}

fn fetch_url_text(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .arg("-sL")
        .arg(url)
        .output()
        .map_err(|e| format!("请求失败: {e}"))?;

    if !output.status.success() {
        return Err(format!("请求失败，HTTP 状态码: {:?}", output.status.code()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_plugin_json(text: &str) -> Result<Vec<PluginConfig>, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| format!("响应不是有效 JSON: {e}"))?;

    let mut imported: Vec<PluginConfig> = Vec::new();

    if let Some(array) = value.as_array() {
        for item in array {
            let plugin: PluginConfig = serde_json::from_value(item.clone())
                .map_err(|e| format!("插件格式错误: {e}"))?;
            imported.push(plugin);
        }
    } else if let Some(plugins) = value.get("plugins").and_then(|v| v.as_array()) {
        for item in plugins {
            let plugin: PluginConfig = serde_json::from_value(item.clone())
                .map_err(|e| format!("插件格式错误: {e}"))?;
            imported.push(plugin);
        }
    } else {
        let plugin: PluginConfig = serde_json::from_value(value)
            .map_err(|e| format!("插件格式错误: {e}"))?;
        imported.push(plugin);
    }

    if imported.is_empty() {
        return Err("没有找到可导入的插件".into());
    }

    Ok(imported)
}

fn merge_imported_plugins(state: &AppState, imported: Vec<PluginConfig>) -> Result<Vec<PluginConfig>, String> {
    let mut plugins = state.plugins.lock().unwrap();
    for plugin in imported {
        if plugin.id.trim().is_empty()
            || plugin.name.trim().is_empty()
            || plugin.command_template.trim().is_empty()
        {
            return Err("插件缺少 id/name/commandTemplate 字段".into());
        }

        if let Some(existing) = plugins.iter_mut().find(|p| p.id == plugin.id) {
            *existing = plugin;
        } else {
            plugins.push(plugin);
        }
    }

    let snapshot = plugins.clone();
    persist_plugins(&state.data_dir, &snapshot)?;
    Ok(snapshot)
}

fn github_raw_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    let candidates = [
        format!("https://raw.githubusercontent.com/{input}/main/dsh-plugin.json"),
        format!("https://raw.githubusercontent.com/{input}/main/plugin.json"),
        format!("https://raw.githubusercontent.com/{input}/master/dsh-plugin.json"),
        format!("https://raw.githubusercontent.com/{input}/master/plugin.json"),
    ];

    for url in candidates {
        if let Ok(text) = fetch_url_text(&url) {
            if text.trim_start().starts_with('{') || text.trim_start().starts_with('[') {
                return Ok(url);
            }
        }
    }

    Err("未能从 GitHub 仓库自动找到 dsh-plugin.json 或 plugin.json".into())
}

fn dshmarket_candidates(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.starts_with("http://") || input.starts_with("https://") {
        vec![input.to_string()]
    } else {
        vec![
            format!("https://dsh.market/api/plugins/{input}"),
            format!("https://dsh.market/api/plugin/{input}"),
            format!("https://dsh.market/plugins/{input}.json"),
            format!("https://dsh.market/plugins/{input}"),
        ]
    }
}

fn is_github_input(input: &str) -> bool {
    let input = input.trim();
    input.starts_with("https://github.com/")
        || input.starts_with("http://github.com/")
        || (input.contains('/') && !input.contains("://") && !input.contains('\\'))
}

fn github_candidate_urls(input: &str) -> Vec<String> {
    let input = input.trim();
    let path = input
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_end_matches('/');

    let path = path
        .replace("https://raw.githubusercontent.com/", "")
        .replace("http://raw.githubusercontent.com/", "");

    let parts: Vec<&str> = path.split('/').collect();
    let mut owner = String::new();
    let mut repo = String::new();
    let mut file_path = String::new();

    if parts.len() >= 2 {
        owner = parts[0].to_string();
        repo = parts[1].trim_end_matches(".git").to_string();

        let mut rest = &parts[2..];
        if rest.first() == Some(&"blob") && rest.len() >= 2 {
            rest = &rest[2..];
        }
        file_path = rest.join("/");
    }

    let mut urls = Vec::new();
    let branches: &[&str] = if file_path.is_empty() {
        &["main", "master"]
    } else {
        &["main", "master"]
    };

    for branch in branches {
        let files: Vec<String> = if file_path.is_empty() {
            vec!["dsh-plugin.json".to_string(), "plugin.json".to_string()]
        } else {
            vec![file_path.clone()]
        };

        for file in files {
            let base = format!("{owner}/{repo}/{branch}/{file}");
            urls.push(format!("https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://cdn.jsdelivr.net/gh/{owner}/{repo}@{branch}/{file}"));
            urls.push(format!("https://ghproxy.net/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://mirror.ghproxy.com/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://ghproxy.cc/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://gh.llkk.cc/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://gh-proxy.com/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://github.moeyy.xyz/https://raw.githubusercontent.com/{base}"));
            urls.push(format!("https://raw.gitmirror.com/{base}"));
        }
    }

    urls
}

fn fetch_url_once(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .arg("-sL")
        .arg("--connect-timeout")
        .arg("5")
        .arg("--max-time")
        .arg("10")
        .arg(url)
        .output()
        .map_err(|e| format!("请求失败: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!("HTTP 状态码: {:?}", output.status.code()))
    }
}


fn fetch_url_with_retry(url: &str) -> Result<String, String> {
    let mut last_error = String::new();

    for attempt in 0..2 {
        let output = Command::new("curl")
            .arg("-sL")
            .arg("--connect-timeout")
            .arg("5")
            .arg("--max-time")
            .arg("12")
            .arg(url)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
            Ok(output) => {
                last_error = format!(
                    "请求失败，HTTP 状态码: {:?} (第 {} 次)",
                    output.status.code(),
                    attempt + 1
                );
            }
            Err(e) => {
                last_error = format!("请求失败: {e} (第 {} 次)", attempt + 1);
            }
        }
    }

    Err(last_error)
}

fn read_plugin_source(input: &str) -> Result<String, String> {
    let input = input.trim();

    if input.starts_with("file://") {
        let path = input.trim_start_matches("file://").trim_start_matches('/');
        return fs::read_to_string(path).map_err(|e| format!("读取本地文件失败: {e}"));
    }

    let is_local_path = input.contains('\\')
        || (input.len() > 2 && input.as_bytes()[1] == b':' && input.contains('/'))
        || input.starts_with("C:/")
        || input.starts_with("D:/");

    if is_local_path {
        return fs::read_to_string(input).map_err(|e| format!("读取本地文件失败: {e}"));
    }

    if is_github_input(input) {
        let candidates = github_candidate_urls(input);
        let mut errors = Vec::new();
        for url in candidates {
            match fetch_url_once(&url) {
                Ok(text) => {
                    if text.trim_start().starts_with('{') || text.trim_start().starts_with('[') {
                        return Ok(text);
                    }
                    errors.push(format!("{url} 返回的不是 JSON"));
                }
                Err(e) => errors.push(format!("{url} 失败: {e}")),
            }
            if errors.len() >= 5 {
                break;
            }
        }
        return Err(format!("GitHub 导入失败：{}", errors.join("；")));
    }

    fetch_url_with_retry(input)
}



#[tauri::command]
fn import_plugin_from_url(state: State<'_, AppState>, url: String) -> Result<Vec<PluginConfig>, String> {
    let text = read_plugin_source(&url)?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("响应不是有效 JSON: {e}"))?;

    let mut imported: Vec<PluginConfig> = Vec::new();

    if let Some(array) = value.as_array() {
        for item in array {
            let plugin: PluginConfig = serde_json::from_value(item.clone())
                .map_err(|e| format!("插件格式错误: {e}"))?;
            imported.push(plugin);
        }
    } else if let Some(plugins) = value.get("plugins").and_then(|v| v.as_array()) {
        for item in plugins {
            let plugin: PluginConfig = serde_json::from_value(item.clone())
                .map_err(|e| format!("插件格式错误: {e}"))?;
            imported.push(plugin);
        }
    } else {
        let plugin: PluginConfig = serde_json::from_value(value)
            .map_err(|e| format!("插件格式错误: {e}"))?;
        imported.push(plugin);
    }

    if imported.is_empty() {
        return Err("没有找到可导入的插件".into());
    }

    let mut plugins = state.plugins.lock().unwrap();
    for plugin in imported {
        if plugin.id.trim().is_empty()
            || plugin.name.trim().is_empty()
            || plugin.command_template.trim().is_empty()
        {
            return Err("插件缺少 id/name/commandTemplate 字段".into());
        }

        if let Some(existing) = plugins.iter_mut().find(|p| p.id == plugin.id) {
            *existing = plugin;
        } else {
            plugins.push(plugin);
        }
    }

    let snapshot = plugins.clone();
    persist_plugins(&state.data_dir, &snapshot)?;
    Ok(snapshot)
}


#[tauri::command]
fn delete_plugin(state: State<'_, AppState>, id: String) -> Result<Vec<PluginConfig>, String> {
    let plugin = {
        let plugins = state.plugins.lock().unwrap();
        plugins.iter().find(|p| p.id == id).cloned()
    };

    {
        let mut processes = state.plugin_processes.lock().unwrap();
        if let Some(mut child) = processes.remove(&id) {
            let pid = child.id();
            #[cfg(windows)]
            {
                kill_process_tree(pid);
                if let Some(plugin) = &plugin {
                    if plugin.command_template.contains("deepseek-ai") {
                        kill_processes_by_commandline("deepseek-ai");
                    }
                }
            }
            #[cfg(not(windows))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }

    let mut plugins = state.plugins.lock().unwrap();
    plugins.retain(|p| p.id != id);
    let snapshot = plugins.clone();
    persist_plugins(&state.data_dir, &snapshot)?;
    Ok(snapshot)
}

#[tauri::command]
fn get_plugin_statuses(state: State<'_, AppState>) -> HashMap<String, PluginStatus> {
    let mut processes = state.plugin_processes.lock().unwrap();
    let mut result = HashMap::new();
    let mut finished = Vec::new();

    for (id, child) in processes.iter_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                finished.push(id.clone());
                result.insert(
                    id.clone(),
                    PluginStatus {
                        running: false,
                        pid: None,
                    },
                );
            }
            Ok(None) => {
                result.insert(
                    id.clone(),
                    PluginStatus {
                        running: true,
                        pid: Some(child.id()),
                    },
                );
            }
            Err(_) => {
                finished.push(id.clone());
                result.insert(
                    id.clone(),
                    PluginStatus {
                        running: false,
                        pid: None,
                    },
                );
            }
        }
    }

    for id in finished {
        processes.remove(&id);
    }

    result
}

#[tauri::command]
fn start_plugin(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<PluginStatus, String> {
    start_plugin_inner(&app, state.inner(), &id)
}

#[tauri::command]
fn stop_plugin(state: State<'_, AppState>, id: String) -> Result<(), String> {
    stop_plugin_inner(state.inner(), &id)
}


#[tauri::command]
fn get_dsh_status(state: State<'_, AppState>) -> DshStatus {
    let mut guard = state.child.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code();
                guard.take();
                DshStatus {
                    running: false,
                    pid: None,
                    state: "exited".into(),
                    exit_code: code,
                }
            }
            Ok(None) => DshStatus {
                running: true,
                pid: Some(child.id()),
                state: "running".into(),
                exit_code: None,
            },
            Err(_) => {
                guard.take();
                DshStatus {
                    running: false,
                    pid: None,
                    state: "error".into(),
                    exit_code: None,
                }
            }
        }
    } else {
        DshStatus {
            running: false,
            pid: None,
            state: "stopped".into(),
            exit_code: None,
        }
    }
}

#[tauri::command]
fn open_dsh_web(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    let url = format!("http://127.0.0.1:{}", settings.port);
    if wait_for_port(settings.port, Duration::from_secs(10)) {
        open_in_browser(&url);
        Ok(())
    } else {
        Err("等待 DSH Web 启动超时，请确认服务是否正常运行".into())
    }
}

#[tauri::command]
fn get_logs(state: State<'_, AppState>) -> Result<String, String> {
    fs::read_to_string(&state.log_file).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_logs(state: State<'_, AppState>) -> Result<(), String> {
    fs::write(&state.log_file, "").map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let settings = load_settings(&data_dir);
    cleanup_logs(&data_dir, settings.log_retention_days);
    let log_file = current_log_file(&data_dir);
    let _ = File::create(&log_file);
    let plugins = load_plugins(&data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            child: Mutex::new(None),
            settings: Mutex::new(settings),
            data_dir,
            log_file,
            tray: Mutex::new(None),
            plugins: Mutex::new(plugins),
            plugin_processes: Mutex::new(HashMap::new()),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.app_handle().state::<AppState>();
                if state.settings.lock().unwrap().minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })

        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let start_i = MenuItem::with_id(app, "start", "启动 DSH", true, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "打开网页", true, None::<&str>)?;
            let stop_i = MenuItem::with_id(app, "stop", "停止 DSH", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &start_i, &open_i, &stop_i, &quit_i])?;

            let tray = TrayIconBuilder::with_id("dsh-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "start" => {
                        let state = app.state::<AppState>();
                        let _ = start_dsh_inner(app, state.inner());
                    }
                    "open" => {
                        let state = app.state::<AppState>();
                        let settings = state.settings.lock().unwrap().clone();
                        open_in_browser(&format!("http://127.0.0.1:{}", settings.port));
                    }
                    "stop" => {
                        let state = app.state::<AppState>();
                        let _ = stop_dsh_inner(state.inner());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
              let state = app.state::<AppState>();
              *state.tray.lock().unwrap() = Some(tray);

              let shortcut: Shortcut = "alt+shift+d"
                  .parse()
                  .map_err(|e| format!("快捷键解析失败: {e}"))?;
              app.global_shortcut()
                  .register(shortcut)
                  .map_err(|e| format!("注册快捷键失败: {e}"))
                    .ok();
              app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
                  if event.state() == ShortcutState::Pressed {
                      if let Some(window) = app.get_webview_window("launcher") {
                          let visible = window.is_visible().unwrap_or(false);
                          if visible {
                              let _ = window.hide();
                          } else {
                              let _ = window.show();
                              let _ = window.set_focus();
                              let _ = window.emit("launcher-show", ());
                          }
                      }
                  }
              }).ok();



              let state = app.state::<AppState>();
              if state.settings.lock().unwrap().auto_start_on_launch {
                  if let Err(e) = start_dsh_inner(app.handle(), state.inner()) {
                        eprintln!("自动启动 DSH 失败: {e}");
                    }
              }

            Ok(())
        })

        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            start_dsh,
            stop_dsh,
            get_dsh_status,
            open_dsh_web,
            get_logs,
            clear_logs,
            get_plugins,
            save_plugin,
            import_plugin_from_url,
            delete_plugin,
            get_plugin_statuses,
            start_plugin,
            stop_plugin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
/* ===== old scaffold below is kept only as a reference and is not compiled =====

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
*/
