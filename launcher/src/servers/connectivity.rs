//! Playit.gg public Minecraft Java tunnels, without installing a separate application.
//! The official signed Windows agent is cached under Valtic's data directory and
//! stays running only while Valtic-hosted Java servers need it.
//! First-time agent claiming requires approval in the user's browser.
use crate::backend;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{atomic::{AtomicU64, Ordering}, LazyLock, Mutex},
    thread,
    time::Duration,
};

const RELEASES: &str = "https://api.github.com/repos/playit-cloud/playit-agent/releases/latest";
const ASSET: &str = "playit-windows-x86_64-signed.exe";
const API: &str = "https://api.playit.gg";
const USER_AGENT: &str = "ValticLauncher/1.0 (https://valticsmp.vercel.app/launcher)";
const MAX_AGENT_SIZE: u64 = 40 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    Checking,
    SetupRequired { reason: String },
    Public { address: String },
    Unavailable { reason: String },
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static CONNECTIONS: LazyLock<Mutex<HashMap<PathBuf, (u64, Connection)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static AGENT: LazyLock<Mutex<Option<Child>>> = LazyLock::new(|| Mutex::new(None));

pub fn current(dir: &Path) -> Option<Connection> {
    CONNECTIONS.lock().ok()?.get(dir).map(|(_, state)| state.clone())
}

fn current_job(dir: &Path, id: u64) -> bool {
    CONNECTIONS.lock().is_ok_and(|entries| entries.get(dir).is_some_and(|(n, _)| *n == id))
}

fn set(dir: &Path, id: u64, value: Connection) -> bool {
    let Ok(mut entries) = CONNECTIONS.lock() else { return false; };
    let Some((n, state)) = entries.get_mut(dir) else { return false; };
    if *n != id { return false; }
    *state = value;
    true
}

pub fn stop(dir: &Path) {
    let empty = if let Ok(mut entries) = CONNECTIONS.lock() {
        entries.remove(dir);
        entries.is_empty()
    } else {
        false
    };
    if empty {
        if let Ok(mut agent) = AGENT.lock() {
            if let Some(mut child) = agent.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn http_client() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(45))
        .build()
}

fn download_agent() -> Result<PathBuf, String> {
    if !cfg!(windows) {
        return Err("Automatic Playit.gg agent setup currently supports Windows.".into());
    }
    let directory = backend::data_dir().join("playit");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let destination = directory.join("playit.exe");
    let release: Value = http_client().get(RELEASES).set("User-Agent", USER_AGENT)
        .call().map_err(|e| format!("Cannot check Playit.gg releases: {e}"))?
        .into_json().map_err(|e| format!("Invalid Playit release metadata: {e}"))?;
    let version = release["tag_name"].as_str().ok_or("Playit release has no version")?;
    let asset = release["assets"].as_array()
        .and_then(|a| a.iter().find(|a| a["name"] == ASSET))
        .ok_or("Playit release does not have a signed Windows agent")?;
    let url = asset["browser_download_url"].as_str().ok_or("Playit download URL missing")?;
    if !url.starts_with("https://github.com/playit-cloud/playit-agent/releases/download/") {
        return Err("Unexpected Playit download source".into());
    }
    let expected = asset["digest"].as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Playit agent has no SHA-256 verification data")?;
    let marker = directory.join("version.txt");
    if destination.is_file() && fs::read_to_string(&marker).ok().as_deref() == Some(version) {
        // A cached exe was already verified when it was downloaded.
        return Ok(destination);
    }
    if asset["size"].as_u64().is_some_and(|n| n > MAX_AGENT_SIZE) {
        return Err("Playit agent download is unexpectedly large".into());
    }
    let download = directory.join("playit.download");
    let result = (|| -> Result<(), String> {
        let mut response = http_client().get(url).set("User-Agent", USER_AGENT)
            .call().map_err(|e| format!("Playit download failed: {e}"))?
            .into_reader().take(MAX_AGENT_SIZE + 1);
        let mut output = File::create(&download).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 16384];
        let mut total = 0u64;
        loop {
            let len = response.read(&mut buffer).map_err(|e| e.to_string())?;
            if len == 0 { break; }
            total += len as u64;
            if total > MAX_AGENT_SIZE { return Err("Playit download exceeded size limit".into()); }
            output.write_all(&buffer[..len]).map_err(|e| e.to_string())?;
            digest.update(&buffer[..len]);
        }
        output.sync_all().map_err(|e| e.to_string())?;
        if format!("{:x}", digest.finalize()) != expected.to_ascii_lowercase() {
            return Err("Playit download checksum mismatch".into());
        }
        if destination.exists() {
            fs::remove_file(&destination).map_err(|e| e.to_string())?;
        }
        fs::rename(&download, &destination).map_err(|e| e.to_string())?;
        fs::write(&marker, version).map_err(|e| e.to_string())
    })();
    if result.is_err() { let _ = fs::remove_file(download); }
    result.map(|()| destination)
}

fn agent_running() -> bool {
    let Ok(mut agent) = AGENT.lock() else { return false; };
    agent.as_mut().is_some_and(|child| child.try_wait().ok() == Some(None))
}

fn ensure_agent() -> Result<(), String> {
    let mut agent = AGENT.lock().map_err(|e| e.to_string())?;
    if agent.as_mut().is_some_and(|child| child.try_wait().ok() == Some(None)) {
        return Ok(());
    }
    let binary = download_agent()?;
    let mut command = Command::new(binary);
    command.arg("-s").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let child = command.spawn().map_err(|e| format!("Could not start Playit.gg: {e}"))?;
    *agent = Some(child);
    Ok(())
}

/// The Playit CLI's portable mode stores its claimed secret here. Never log it or
/// put it in Valtic settings or copy it to another directory.
fn secret() -> Option<String> {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)?.join("playit_gg").join("playit.toml");
    let source = fs::read_to_string(path).ok()?;
    let token = toml::from_str::<toml::Value>(&source).ok()?
        .get("secret_key")?.as_str()?.trim().to_owned();
    (token.len() >= 32 && token.bytes().all(|b| b.is_ascii_hexdigit())).then_some(token)
}

fn request(secret: &str, endpoint: &str, data: Value) -> Result<Value, String> {
    let result: Value = http_client().post(&format!("{API}{endpoint}"))
        .set("User-Agent", USER_AGENT)
        .set("Authorization", &format!("Agent-Key {secret}"))
        .send_json(data)
        .map_err(|e| format!("Playit API unavailable: {e}"))?
        .into_json().map_err(|e| format!("Unexpected Playit response: {e}"))?;
    if result["status"] != "success" {
        return Err(format!("Playit API response: {}", result["data"]));
    }
    Ok(result["data"].clone())
}

/// Match on local port, not on the first tunnel in the account. Display only
/// the address Playit actually assigns, never a made-up example hostname.
fn find_tunnel(data: &Value, port: u16) -> Option<String> {
    let tunnels = data["tunnels"].as_array()?;
    tunnels.iter().find_map(|tunnel| {
        if tunnel["disabled_reason"].is_string() || tunnel["port_type"] == "udp" {
            return None;
        }
        let fields = tunnel["agent_config"]["fields"].as_array()?;
        let configured = fields.iter().find_map(|field| {
            (field["name"] == "local_port").then(|| field["value"].as_str()?.parse::<u16>().ok()).flatten()
        });
        // The agent defaults to 25565 when no port override is configured.
        if configured.or((tunnels.len() == 1 && port == 25565).then_some(25565)) != Some(port) {
            return None;
        }
        tunnel["display_address"].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
    })
}

fn ensure_tunnel(key: &str, port: u16) -> Result<Option<String>, String> {
    let data = request(key, "/v1/agents/rundata", json!({}))?;
    if let Some(address) = find_tunnel(&data, port) {
        return Ok(Some(address));
    }
    let title = format!("Valtic {port}");
    let tunnels = request(key, "/v1/tunnels/list", json!({}))?;
    let exists = tunnels["tunnels"].as_array().is_some_and(|all| all.iter().any(|t| {
        t["name"] == title && t["tunnel_type"] == "minecraft-java"
    }));
    if !exists {
        request(key, "/v1/tunnels/create", json!({
            "name": title,
            "protocol": { "type": "tunnel-type", "details": "minecraft-java" },
            "origin": { "type": "agent", "data": {
                "agent_id": data["agent_id"], "config": { "fields": [
                    { "name": "local_ip", "value": "127.0.0.1" },
                    { "name": "local_port", "value": port.to_string() }
                ] }
            } },
            "endpoint": { "type": "region", "details": { "region": "global", "port": null } },
            "enabled": true, "firewall_id": null
        }))?;
    }
    Ok(None)
}

pub fn start(dir: PathBuf, port: u16) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.insert(dir.clone(), (id, Connection::Checking));
    } else { return; }
    thread::spawn(move || {
        if let Err(reason) = ensure_agent() {
            let _ = set(&dir, id, Connection::Unavailable { reason });
            return;
        }
        let mut created = false;
        let mut failures = 0u8;
        while current_job(&dir, id) && super::java::is_running(&dir) {
            if !agent_running() {
                let _ = set(&dir, id, Connection::Unavailable {
                    reason: "Playit stopped unexpectedly. Press Retry to restart it.".into()
                });
                return;
            }
            let Some(key) = secret() else {
                let _ = set(&dir, id, Connection::SetupRequired {
                    reason: "Approve the Playit.gg agent in the browser window opened by Valtic. This is required once per computer.".into()
                });
                thread::sleep(Duration::from_secs(2));
                continue;
            };
            if !created { let _ = set(&dir, id, Connection::Checking); }
            let result = if !created { ensure_tunnel(&key, port) }
                else { request(&key, "/v1/agents/rundata", json!({}))
                    .map(|d| find_tunnel(&d, port)) };
            match result {
                Ok(Some(address)) => {
                    let _ = set(&dir, id, Connection::Public { address });
                    failures = 0;
                    created = true;
                }
                Ok(None) => {
                    created = true;
                    let _ = set(&dir, id, Connection::Checking);
                    failures = 0;
                }
                Err(reason) => {
                    failures = failures.saturating_add(1);
                    if failures > 3 {
                        let _ = set(&dir, id, Connection::Unavailable {
                            reason: format!("{reason}. Check your Playit account or press Retry.")
                        });
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_address_without_a_real_matching_port() {
        let data = json!({"tunnels":[
            {"port_type":"tcp","display_address":"example.playit.gg",
             "disabled_reason":null,"agent_config":{"fields":[{"name":"local_port","value":"25566"}]}}
        ]});
        assert_eq!(find_tunnel(&data, 25565), None);
        assert_eq!(find_tunnel(&data, 25566).as_deref(), Some("example.playit.gg"));
    }
}
