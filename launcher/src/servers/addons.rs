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
    fn unsupported_software_is_not_offered_addons() {
        assert!(!supports_voice_chat(Software::Vanilla));
        assert!(!supports_geyser(Software::Vanilla));
        assert!(!supports_geyser(Software::Fabric));
        assert!(supports_geyser(Software::Paper));
        assert!(supports_voice_chat(Software::Forge));
    }
}
