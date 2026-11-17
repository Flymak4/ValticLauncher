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
    io::{BufRead, BufReader, Read, Write},
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
    Public {
        address: String,
        voice_address: Option<String>,
        geyser_address: Option<String>,
        voice_verified: bool,
        geyser_verified: bool,
    },
    Unavailable { reason: String },
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static CONNECTIONS: LazyLock<Mutex<HashMap<PathBuf, (u64, Connection)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static AGENT: LazyLock<Mutex<Option<Child>>> = LazyLock::new(|| Mutex::new(None));
static CLAIM_URL: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

/// This code is only a public one-time approval URL, never the agent secret.
pub fn claim_url() -> Option<String> {
    CLAIM_URL.lock().ok()?.clone()
}

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
    command.arg("-s").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|e| format!("Could not start Playit.gg: {e}"))?;
    if let Ok(mut link) = CLAIM_URL.lock() { *link = None; }
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(start) = line.find("https://playit.gg/claim/") {
                    let rest = &line[start + "https://playit.gg/claim/".len()..];
                    let code: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
                    if code.len() == 10 {
                        if let Ok(mut link) = CLAIM_URL.lock() {
                            *link = Some(format!("https://playit.gg/claim/{code}"));
                        }
                    }
                }
            }
        });
    }
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

/// The public endpoints allocated to one server by Playit.
#[derive(Clone, Debug)]
pub struct TunnelInfo {
    pub java_address: String,
    pub voice_address: Option<String>,
    pub geyser_address: Option<String>,
}

fn server_tag(dir: &Path) -> String {
    let raw = dir.file_name().and_then(|s| s.to_str()).unwrap_or("server");
    let safe: String = raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(24).collect();
    safe.trim_matches('-').to_string()
}

fn find_tunnel(data: &Value, name: &str, port: u16, port_type: &str) -> Option<String> {
    data["tunnels"].as_array()?.iter().find_map(|tunnel| {
        if tunnel["name"].as_str() != Some(name)
            || tunnel["port_type"].as_str() != Some(port_type)
            || tunnel["disabled_reason"].is_string()
        {
            return None;
        }
        let fields = tunnel["agent_config"]["fields"].as_array()?;
        let configured = fields.iter().find_map(|field| {
            (field["name"] == "local_port")
                .then(|| field["value"].as_str()?.parse::<u16>().ok())
                .flatten()
        });
        if configured != Some(port) {
            return None;
        }
        tunnel["display_address"].as_str()
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
    })
}

fn account_has_tunnel(data: &Value, name: &str, port_type: &str) -> bool {
    data["tunnels"].as_array().is_some_and(|tunnels| tunnels.iter().any(|tunnel| {
        tunnel["name"].as_str() == Some(name)
            && tunnel["port_type"].as_str() == Some(port_type)
    }))
}

fn create_tunnel(
    key: &str,
    agent_id: &Value,
    name: &str,
    port: u16,
    port_type: &str,
    protocol: Value,
) -> Result<(), String> {
    request(key, "/v1/tunnels/create", json!({
        "name": name,
        "protocol": protocol,
        "origin": { "type": "agent", "data": {
            "agent_id": agent_id,
            "config": { "fields": [
                { "name": "local_ip", "value": "127.0.0.1" },
                { "name": "local_port", "value": port.to_string() }
            ] }
        } },
        "endpoint": { "type": "region", "details": { "region": "global", "port": null } },
        "enabled": true,
        "firewall_id": null
    }))?;
    let _ = port_type;
    Ok(())
}

fn ensure_java_tunnel(key: &str, run_data: &Value, dir: &Path, port: u16) -> Result<(), String> {
    let name = format!("Valtic {} Java", server_tag(dir));
    if find_tunnel(run_data, &name, port, "tcp").is_some() {
        return Ok(());
    }
    let tunnels = request(key, "/v1/tunnels/list", json!({}))?;
    if account_has_tunnel(&tunnels, &name, "tcp") {
        return Ok(());
    }
    create_tunnel(
        key, &run_data["agent_id"], &name, port, "tcp",
        json!({ "type": "tunnel-type", "details": "minecraft-java" }),
    )
}

fn ensure_udp_tunnel(
    key: &str,
    run_data: &Value,
    dir: &Path,
    port: u16,
    suffix: &str,
    description: &str,
) -> Result<(), String> {
    let name = format!("Valtic {} {suffix}", server_tag(dir));
    if find_tunnel(run_data, &name, port, "udp").is_some() {
        return Ok(());
    }
    let tunnels = request(key, "/v1/tunnels/list", json!({}))?;
    if account_has_tunnel(&tunnels, &name, "udp") {
        return Ok(());
    }
    create_tunnel(
        key, &run_data["agent_id"], &name, port, "udp",
        json!({
            "type": "raw-ports",
            "details": {
                "port_type": "udp",
                "port_count": 1,
                "software_description": description
            }
        }),
    )
}

/// Creates the Java and optional UDP tunnels before the game process starts. If this is the
/// first run, the player approves the Playit agent once in their browser, then presses Start again.
pub fn prepare(dir: &Path) -> Result<TunnelInfo, String> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.insert(dir.to_path_buf(), (id, Connection::Checking));
    } else {
        return Err("Playit connection state is unavailable.".into());
    }

    let result = (|| -> Result<TunnelInfo, String> {
        ensure_agent()?;
        let Some(key) = secret() else {
            let reason = "Approve the Playit.gg agent in the browser. This is required once per computer; after approval, press Start again.".to_string();
            let _ = set(dir, id, Connection::SetupRequired { reason: reason.clone() });
            return Err(reason);
        };
        let meta = super::java::metadata(dir)?;
        let initial = request(&key, "/v1/agents/rundata", json!({}))?;
        ensure_java_tunnel(&key, &initial, dir, meta.port)?;
        if meta.voice_chat {
            ensure_udp_tunnel(&key, &initial, dir, meta.voice_port, "Voice", "Valtic Simple Voice Chat UDP")?;
        }
        if meta.geyser {
            ensure_udp_tunnel(&key, &initial, dir, meta.geyser_port, "Bedrock", "Valtic Geyser Bedrock UDP")?;
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(45);
        loop {
            if !agent_running() {
                return Err("The Playit agent stopped while setting up tunnels.".into());
            }
            let data = request(&key, "/v1/agents/rundata", json!({}))?;
            let java_address = find_tunnel(
                &data, &format!("Valtic {} Java", server_tag(dir)), meta.port, "tcp"
            );
            let voice_address = if meta.voice_chat {
                find_tunnel(&data, &format!("Valtic {} Voice", server_tag(dir)), meta.voice_port, "udp")
            } else { None };
            let geyser_address = if meta.geyser {
                find_tunnel(&data, &format!("Valtic {} Bedrock", server_tag(dir)), meta.geyser_port, "udp")
            } else { None };

            if let Some(java_address) = java_address {
                if (!meta.voice_chat || voice_address.is_some())
                    && (!meta.geyser || geyser_address.is_some())
                {
                    return Ok(TunnelInfo { java_address, voice_address, geyser_address });
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err("Playit created the tunnel, but it has not become active yet. Check your Playit account and retry.".into());
            }
            thread::sleep(Duration::from_secs(2));
        }
    })();

    match &result {
        Ok(info) => {
            let _ = set(dir, id, Connection::Public {
                address: info.java_address.clone(),
                voice_address: info.voice_address.clone(),
                geyser_address: info.geyser_address.clone(),
                voice_verified: false,
                geyser_verified: false,
            });
        }
        Err(reason) => {
            if !matches!(current(dir), Some(Connection::SetupRequired { .. })) {
                let _ = set(dir, id, Connection::Unavailable { reason: reason.clone() });
            }
        }
    }
    result
}

/// Polls the live agent and checks the installed config plus the plugin's startup log. A green
/// tick is shown only after both the public UDP tunnel and the server-side service are confirmed.
pub fn monitor(dir: PathBuf, info: TunnelInfo) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.insert(dir.clone(), (id, Connection::Public {
            address: info.java_address.clone(),
            voice_address: info.voice_address.clone(),
            geyser_address: info.geyser_address.clone(),
            voice_verified: false,
            geyser_verified: false,
        }));
    } else {
        return;
    }

    thread::spawn(move || {
        while current_job(&dir, id) && super::java::is_running(&dir) {
            if !agent_running() {
                let _ = set(&dir, id, Connection::Unavailable {
                    reason: "Playit stopped unexpectedly. Press Retry Playit to reconnect.".into()
                });
                return;
            }
            let Some(key) = secret() else {
                let _ = set(&dir, id, Connection::SetupRequired {
                    reason: "The Playit agent needs approval in your browser.".into()
                });
                thread::sleep(Duration::from_secs(3));
                continue;
            };
            let meta = match super::java::metadata(&dir) {
                Ok(meta) => meta,
                Err(reason) => {
                    let _ = set(&dir, id, Connection::Unavailable { reason });
                    return;
                }
            };
            let data = match request(&key, "/v1/agents/rundata", json!({})) {
                Ok(data) => data,
                Err(_) => {
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };
            let java_ok = find_tunnel(
                &data, &format!("Valtic {} Java", server_tag(&dir)), meta.port, "tcp"
            ).is_some();
            let voice_tunnel_ok = !meta.voice_chat || find_tunnel(
                &data, &format!("Valtic {} Voice", server_tag(&dir)), meta.voice_port, "udp"
            ).is_some();
            let geyser_tunnel_ok = !meta.geyser || find_tunnel(
                &data, &format!("Valtic {} Bedrock", server_tag(&dir)), meta.geyser_port, "udp"
            ).is_some();
            let log = fs::read_to_string(dir.join("logs/valtic-console.log")).unwrap_or_default();
            let (voice_config_ok, geyser_config_ok) = super::addons::network_config_matches(
                &dir, meta.software, meta.voice_port, info.voice_address.as_deref(), meta.geyser_port
            );
            let voice_loaded = log.contains("Voice chat server started");
            let geyser_loaded = log.contains("Geyser-Spigot")
                && (log.contains("Done (") || log.contains("Enabled"));
            let voice_verified = meta.voice_chat
                && voice_tunnel_ok && voice_config_ok && voice_loaded;
            let geyser_verified = meta.geyser
                && geyser_tunnel_ok && geyser_config_ok && geyser_loaded;

            if !java_ok {
                let _ = set(&dir, id, Connection::Unavailable {
                    reason: "The Playit Java tunnel is not active.".into()
                });
                return;
            }
            let _ = set(&dir, id, Connection::Public {
                address: info.java_address.clone(),
                voice_address: info.voice_address.clone(),
                geyser_address: info.geyser_address.clone(),
                voice_verified,
                geyser_verified,
            });
            thread::sleep(Duration::from_secs(5));
        }
    });
}

/// Retry Playit setup for a running Java server.
pub fn start(dir: PathBuf, _port: u16) {
    thread::spawn(move || {
        let Ok(info) = prepare(&dir) else { return; };
        let Ok(meta) = super::java::metadata(&dir) else { return; };
        if super::java::is_running(&dir) {
            let _ = super::java::stop(&dir);
            for _ in 0..150 {
                if !super::java::is_running(&dir) { break; }
                thread::sleep(Duration::from_millis(100));
            }
            if super::java::is_running(&dir) {
                let _ = set(&dir, NEXT_ID.load(Ordering::Relaxed), Connection::Unavailable {
                    reason: "The server did not stop in time to apply the new UDP tunnel settings. Stop it and retry.".into()
                });
                return;
            }
        }
        if let Err(reason) = super::addons::configure_network(
            &dir, meta.software, meta.voice_port,
            info.voice_address.as_deref(), meta.geyser_port
        ) {
            let _ = set(&dir, NEXT_ID.load(Ordering::Relaxed), Connection::Unavailable { reason });
            return;
        }
        if !super::java::is_running(&dir) {
            if let Err(problem) = super::java::start(&dir) {
                let _ = set(&dir, NEXT_ID.load(Ordering::Relaxed), Connection::Unavailable {
                    reason: problem.message
                });
                return;
            }
        }
        monitor(dir, info);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_only_the_requested_tunnel_and_protocol() {
        let data = json!({"tunnels":[
            {"name":"Valtic test Java","port_type":"tcp","display_address":"java.example:25565",
             "disabled_reason":null,"agent_config":{"fields":[{"name":"local_port","value":"25565"}]}},
            {"name":"Valtic test Voice","port_type":"udp","display_address":"voice.example:30111",
             "disabled_reason":null,"agent_config":{"fields":[{"name":"local_port","value":"24454"}]}}
        ]});
        assert_eq!(find_tunnel(&data, "Valtic test Java", 25565, "tcp").as_deref(), Some("java.example:25565"));
        assert_eq!(find_tunnel(&data, "Valtic test Voice", 24454, "udp").as_deref(), Some("voice.example:30111"));
        assert_eq!(find_tunnel(&data, "Valtic test Voice", 24454, "tcp"), None);
    }
}
