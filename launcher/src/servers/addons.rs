//! Optional Minecraft server additions installed during creation.
//! Downloads only from Modrinth and the official GeyserMC downloads service.
//! These features require UDP connectivity beyond the Java server's TCP tunnel.
use super::java::{self, Software};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Addons {
    pub voice_chat: bool,
    pub geyser: bool,
}

pub fn supports_voice_chat(software: Software) -> bool {
    !matches!(software, Software::Vanilla)
}

/// For now, use the well-supported Paper plugin + Floodgate combination.
/// Do not promise fully automatic Bedrock auth for other mod loaders.
pub fn supports_geyser(software: Software) -> bool {
    matches!(software, Software::Paper)
}

fn modrinth_jar(dir: &Path, project: &str, version: &str, loader: &str, filename: &str) -> Result<(), String> {
    // Minecraft version and loader come from the validated server choice, not arbitrary URLs.
    if !version.bytes().all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c)) {
        return Err("Invalid Minecraft version for add-on lookup.".into());
    }
    let url = format!(
        "https://api.modrinth.com/v2/project/{project}/version?loaders=%5B%22{loader}%22%5D&game_versions=%5B%22{version}%22%5D"
    );
    let versions = java::fetch_json(&url)?;
    let selected = versions.as_array()
        .and_then(|v| v.iter().find(|v| v["version_type"] == "release")
            .or_else(|| v.first()))
        .ok_or_else(|| format!("{project} has no compatible {loader} build for Minecraft {version}"))?;
    let files = selected["files"].as_array().ok_or("Modrinth version has no files")?;
    let selected_file = files.iter()
        .find(|f| f["primary"] == true && f["filename"].as_str().is_some_and(|s| s.ends_with(".jar")))
        .or_else(|| files.iter().find(|f| f["filename"].as_str().is_some_and(|s| s.ends_with(".jar"))))
        .ok_or("Modrinth version has no jar file")?;
    let url = selected_file["url"].as_str().ok_or("Modrinth download URL is missing")?;
    if !url.starts_with("https://cdn.modrinth.com/") {
        return Err("Unexpected Modrinth download host.".into());
    }
    let checksum = selected_file["hashes"]["sha1"].as_str()
        .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Modrinth did not provide a valid SHA-1 checksum")?;
    java::download(url, &dir.join(filename), Some(checksum))
}

fn geyser_jar(dir: &Path, project: &str, platform: &str, filename: &str) -> Result<(), String> {
    let data = java::fetch_json(&format!(
        "https://download.geysermc.org/v2/projects/{project}/versions/latest/builds/latest"
    ))?;
    let sha256 = data["downloads"][platform]["sha256"].as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| format!("No verified {platform} download is available for {project}"))?;
    let url = format!(
        "https://download.geysermc.org/v2/projects/{project}/versions/latest/builds/latest/downloads/{platform}"
    );
    let dest = dir.join(filename);
    java::download(&url, &dest, None)?;
    let verify = (|| -> Result<(), String> {
        let mut file = File::open(&dest).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let len = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if len == 0 { break; }
            hash.update(&buffer[..len]);
        }
        if format!("{:x}", hash.finalize()) != sha256.to_ascii_lowercase() {
            return Err(format!("{project} checksum mismatch"));
        }
        Ok(())
    })();
    if verify.is_err() {
        let _ = fs::remove_file(&dest);
    }
    verify
}


/// Write or update a Java properties key without discarding the plugin's other settings.
fn set_property(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let old = fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{key}=");
    let mut found = false;
    let mut lines = Vec::new();
    for line in old.lines() {
        if line.trim_start().starts_with(&prefix) {
            if !found {
                lines.push(format!("{key}={value}"));
                found = true;
            }
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        lines.push(format!("{key}={value}"));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, format!("{}\n", lines.join("\n"))).map_err(|e| e.to_string())
}

/// Keep Geyser's generated YAML intact while setting the three network values Valtic owns.
fn configure_geyser(path: &Path, port: u16) -> Result<(), String> {
    let old = fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = if old.is_empty() {
        vec!["bedrock:".into()]
    } else {
        old.lines().map(str::to_owned).collect()
    };
    let start = lines.iter().position(|line| line.trim() == "bedrock:");
    let start = match start {
        Some(index) => index,
        None => {
            lines.push("bedrock:".into());
            lines.len() - 1
        }
    };
    let end = lines.iter().enumerate().skip(start + 1)
        .find(|(_, line)| !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#'))
        .map(|(index, _)| index).unwrap_or(lines.len());
    for (key, value) in [
        ("address", "0.0.0.0".to_string()),
        ("port", port.to_string()),
        ("clone-remote-port", "false".to_string()),
    ] {
        let prefix = format!("  {key}:");
        if let Some(index) = (start + 1..end).find(|&index| lines[index].trim_start().starts_with(&prefix.trim_start())) {
            lines[index] = format!("  {key}: {value}");
        } else {
            lines.insert(end.min(lines.len()), format!("  {key}: {value}"));
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, format!("{}\n", lines.join("\n"))).map_err(|e| e.to_string())
}

/// Apply the public UDP endpoints before Java starts, so plugins bind to the ports
/// Playit forwards and Simple Voice Chat advertises its actual public endpoint.
pub fn configure_network(
    dir: &Path,
    software: Software,
    voice_port: u16,
    voice_host: Option<&str>,
    geyser_port: u16,
) -> Result<(), String> {
    if voice_port > 0 {
        let path = if software == Software::Paper {
            dir.join("plugins/voicechat/voicechat-server.properties")
        } else {
            dir.join("config/voicechat/voicechat-server.properties")
        };
        set_property(&path, "port", &voice_port.to_string())?;
        // Do not overwrite a working public UDP address when Playit's API is unavailable.
        if let Some(host) = voice_host.filter(|host| !host.trim().is_empty()) {
            set_property(&path, "bind_address", "")?;
            set_property(&path, "voice_host", host)?;
        }
    }
    if geyser_port > 0 {
        configure_geyser(&dir.join("plugins/Geyser-Spigot/config.yml"), geyser_port)?;
    }
    Ok(())
}

/// A conservative local check; the Playit tunnel and plugin startup are checked separately.
pub fn network_config_matches(
    dir: &Path,
    software: Software,
    voice_port: u16,
    voice_host: Option<&str>,
    geyser_port: u16,
) -> (bool, bool) {
    let voice_ok = if let Some(host) = voice_host {
        let path = if software == Software::Paper {
            dir.join("plugins/voicechat/voicechat-server.properties")
        } else {
            dir.join("config/voicechat/voicechat-server.properties")
        };
        fs::read_to_string(path).ok().is_some_and(|text| {
            text.lines().any(|l| l.trim() == format!("port={voice_port}"))
                && text.lines().any(|l| l.trim() == format!("voice_host={host}"))
        })
    } else {
        false
    };
    let geyser_ok = if geyser_port > 0 {
        fs::read_to_string(dir.join("plugins/Geyser-Spigot/config.yml")).ok().is_some_and(|text| {
            text.lines().any(|l| l.trim() == format!("port: {geyser_port}"))
                && text.lines().any(|l| l.trim() == "address: 0.0.0.0")
                && text.lines().any(|l| l.trim() == "clone-remote-port: false")
        })
    } else {
        false
    };
    (voice_ok, geyser_ok)
}

pub fn install(dir: &Path, software: Software, version: &str, addons: Addons, log: &dyn Fn(String)) -> Result<(), String> {
    if addons.voice_chat && !supports_voice_chat(software) {
        return Err("Voice chat needs Paper, Fabric, Forge or NeoForge (not Vanilla).".into());
    }
    if addons.geyser && !supports_geyser(software) {
        return Err("Automatic Geyser + Floodgate setup currently requires Paper.".into());
    }
    if !addons.voice_chat && !addons.geyser {
        return Ok(());
    }
    let location = if software == Software::Paper { "plugins" } else { "mods" };
    let addon_dir = dir.join(location);
    fs::create_dir_all(&addon_dir).map_err(|e| e.to_string())?;
    if addons.voice_chat {
        let loader = match software {
            Software::Vanilla => unreachable!("checked above"),
            Software::Paper => "bukkit",
            Software::Fabric => "fabric",
            Software::Forge => "forge",
            Software::Neoforge => "neoforge",
        };
        log(format!("Installing Simple Voice Chat for {version} ({loader})"));
        modrinth_jar(&addon_dir, "simple-voice-chat", version, loader, "simple-voice-chat.jar")?;
        log("Voice chat installed. Players need the matching client mod and UDP access (default 24454).".into());
    }
    if addons.geyser {
        log("Installing official Geyser and Floodgate plugins".into());
        geyser_jar(&addon_dir, "geyser", "spigot", "Geyser-Spigot.jar")?;
        geyser_jar(&addon_dir, "floodgate", "spigot", "floodgate-spigot.jar")?;
        // On Paper, Geyser detects the locally installed Floodgate and creates its
        // configuration at first start. Do not write a truncated config.yml.
        log("Geyser and Floodgate installed. Bedrock needs a reachable UDP port (default 19132).".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_udp_tunnel_preserves_configured_voice_host() {
        let dir = std::env::temp_dir().join(format!("valtic-voice-host-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        configure_network(&dir, Software::Paper, 24454, Some("voice.example:43122"), 0).unwrap();
        configure_network(&dir, Software::Paper, 24454, None, 0).unwrap();
        let content = std::fs::read_to_string(dir.join("plugins/voicechat/voicechat-server.properties")).unwrap();
        assert!(content.lines().any(|s| s == "port=24454"));
        assert!(content.lines().any(|s| s == "bind_address="));
        assert!(content.lines().any(|s| s == "voice_host=voice.example:43122"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_software_is_not_offered_addons() {
        assert!(!supports_voice_chat(Software::Vanilla));
        assert!(!supports_geyser(Software::Vanilla));
        assert!(!supports_geyser(Software::Fabric));
        assert!(supports_geyser(Software::Paper));
        assert!(supports_voice_chat(Software::Forge));
    }
}
