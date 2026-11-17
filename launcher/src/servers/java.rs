//! Java Edition server hosting for Valtic. A separate runtime from the legacy native server.
//! All downloads are from official Mojang/Fabric/Forge/NeoForge endpoints.
use super::{Entry, NewServer, Problem, Status, World};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const META: &str = "valtic-server.json";
const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const USER_AGENT: &str = "ValticLauncher/1.0 (https://valticsmp.vercel.app/launcher)";
const MAX_FILE_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Software {
    Vanilla,
    Paper,
    Fabric,
    Forge,
    Neoforge,
}
impl Software {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Paper => "Paper",
            Self::Fabric => "Fabric",
            Self::Forge => "Forge",
            Self::Neoforge => "NeoForge",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "vanilla" => Some(Self::Vanilla),
            "paper" => Some(Self::Paper),
            "fabric" => Some(Self::Fabric),
            "forge" => Some(Self::Forge),
            "neoforge" => Some(Self::Neoforge),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub software: Software,
    pub minecraft_version: String,
    pub loader_version: Option<String>,
    pub ram_gb: u8,
    pub port: u16,
    #[serde(default)]
    pub voice_chat: bool,
    #[serde(default)]
    pub geyser: bool,
    #[serde(default)]
    pub voice_port: u16,
    #[serde(default)]
    pub geyser_port: u16,
    pub launch: Launch,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "path", rename_all = "snake_case")]
pub enum Launch {
    Jar(String),
    Argfile(String),
}
struct Running {
    child: Child,
    started: Instant,
}
static RUNNING: LazyLock<Mutex<HashMap<PathBuf, Running>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn is_java(dir: &Path) -> bool {
    dir.join(META).is_file()
}
pub fn metadata(dir: &Path) -> Result<Metadata, String> {
    let data = fs::read_to_string(dir.join(META)).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| format!("Invalid server metadata: {e}"))
}
pub(super) fn fetch_json(url: &str) -> Result<Value, String> {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Could not fetch {url}: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())
}
fn fetch_text(url: &str) -> Result<String, String> {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}
pub(super) fn download(url: &str, dest: &Path, sha1: Option<&str>) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("The server download must use HTTPS.".into());
    }
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(180))
        .build()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    if response.header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|n| n > MAX_FILE_BYTES)
    {
        return Err("The server download is unexpectedly large.".into());
    }
    let temp = dest.with_extension("download");
    let result = (|| -> Result<(), String> {
        let mut output = File::create(&temp).map_err(|e| e.to_string())?;
        let mut input = response.into_reader().take(MAX_FILE_BYTES + 1);
        let copied = std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
        if copied > MAX_FILE_BYTES {
            return Err("The server download exceeded the size limit.".into());
        }
        output.sync_all().map_err(|e| e.to_string())?;
        if let Some(expected) = sha1 {
            let mut file = File::open(&temp).map_err(|e| e.to_string())?;
            let mut hash = Sha1::new();
            let mut buffer = [0u8; 8192];
            loop {
                let len = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if len == 0 { break; }
                hash.update(&buffer[..len]);
            }
            if format!("{:x}", hash.finalize()) != expected.to_ascii_lowercase() {
                return Err("Mojang server checksum mismatch. Download cancelled.".into());
            }
        }
        fs::rename(&temp, dest).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn versions() -> Result<Vec<String>, String> {
    let v = fetch_json(MANIFEST)?;
    Ok(v["versions"].as_array().ok_or("Invalid version manifest")?
        .iter().filter(|item| item["type"] == "release")
        .filter_map(|item| item["id"].as_str().map(str::to_owned))
        .take(150).collect())
}
fn validate_version(version: &str) -> Result<Value, String> {
    if version.len() > 40 || !version.chars().all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c)) {
        return Err("Invalid Minecraft version.".into());
    }
    let manifest = fetch_json(MANIFEST)?;
    let record = manifest["versions"].as_array()
        .and_then(|a| a.iter().find(|v| v["id"] == version && v["type"] == "release"))
        .ok_or_else(|| format!("Minecraft version {version} is not a released Java version."))?;
    let url = record["url"].as_str().ok_or("Mojang version URL missing")?;
    if !url.starts_with("https://piston-meta.mojang.com/") {
        return Err("Unexpected Mojang metadata URL.".into());
    }
    fetch_json(url)
}
fn install(software: Software, version: &str, dir: &Path, log: &dyn Fn(String)) -> Result<(Launch, Option<String>), String> {
    let version_meta = validate_version(version)?;
    match software {
        Software::Vanilla => {
            let server = &version_meta["downloads"]["server"];
            let url = server["url"].as_str().ok_or("This Minecraft version has no server download.")?;
            if !url.starts_with("https://piston-data.mojang.com/") && !url.starts_with("https://launcher.mojang.com/") {
                return Err("Unrecognised Mojang server download host.".into());
            }
            log(format!("Downloading official Minecraft {version} server"));
            download(url, &dir.join("server.jar"), server["sha1"].as_str())?;
            Ok((Launch::Jar("server.jar".into()), None))
        }
        Software::Paper => {
            // PaperMC v3 Fill download service. Only install a STABLE build for
            // the *requested* Minecraft version; do not silently switch versions.
            let builds = fetch_json(&format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{version}/builds"
            ))?;
            let stable = builds.as_array()
                .and_then(|a| a.iter().find(|b| b["channel"] == "STABLE"))
                .ok_or_else(|| format!("No stable Paper build for Minecraft {version}"))?;
            let data = &stable["downloads"]["server:default"];
            let url = data["url"].as_str().ok_or("Paper build has no download URL")?;
            if !url.starts_with("https://fill-data.papermc.io/") {
                return Err("Unexpected Paper download host.".into());
            }
            let checksum = data["checksums"]["sha256"].as_str()
                .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or("Paper build checksum is missing or invalid")?;
            log(format!("Downloading stable Paper server for Minecraft {version}"));
            download(url, &dir.join("server.jar"), None)?;
            // Verify the official SHA-256 before starting an untrusted jar.
            use sha2::{Digest as _, Sha256};
            let mut file = File::open(dir.join("server.jar")).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut bytes = [0u8; 8192];
            loop {
                let len = file.read(&mut bytes).map_err(|e| e.to_string())?;
                if len == 0 { break; }
                hash.update(&bytes[..len]);
            }
            if format!("{:x}", hash.finalize()) != checksum.to_ascii_lowercase() {
                return Err("Paper server checksum mismatch.".into());
            }
            Ok((Launch::Jar("server.jar".into()), None))
        }
        Software::Fabric => {
            let url = format!("https://meta.fabricmc.net/v2/versions/loader/{version}");
            let loaders = fetch_json(&url)?;
            let loader = loaders.as_array().and_then(|v| v.iter().find(|v| v["loader"]["stable"] == true))
                .or_else(|| loaders.as_array().and_then(|v| v.first()))
                .and_then(|v| v["loader"]["version"].as_str())
                .ok_or_else(|| format!("No Fabric loader supports Minecraft {version}"))?;
            let installers = fetch_json("https://meta.fabricmc.net/v2/versions/installer")?;
            let installer = installers.as_array().and_then(|v| v.iter().find(|v| v["stable"] == true))
                .or_else(|| installers.as_array().and_then(|v| v.first()))
                .and_then(|v| v["version"].as_str()).ok_or("No Fabric installer available")?;
            let url = format!("https://meta.fabricmc.net/v2/versions/loader/{version}/{loader}/{installer}/server/jar");
            log(format!("Downloading Fabric loader {loader} for {version}"));
            download(&url, &dir.join("server.jar"), None)?;
            Ok((Launch::Jar("server.jar".into()), Some(loader.into())))
        }
        Software::Forge => {
            let promos = fetch_json("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")?;
            let build = promos["promos"][format!("{version}-recommended")].as_str()
                .or_else(|| promos["promos"][format!("{version}-latest")].as_str())
                .ok_or_else(|| format!("Forge is not available for Minecraft {version}"))?;
            let release = format!("{version}-{build}");
            let url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{release}/forge-{release}-installer.jar");
            log(format!("Installing Forge {release}"));
            download(&url, &dir.join("installer.jar"), None)?;
            run_installer(dir, log)?;
            let args = find_args(dir, "net/minecraftforge/forge", &release)
                .ok_or("Forge did not create the required launch arguments.")?;
            Ok((Launch::Argfile(args), Some(build.into())))
        }
        Software::Neoforge => {
            let xml = fetch_text("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")?;
            let prefix = if let Some(rest) = version.strip_prefix("1.") { rest } else { version };
            let prefix = format!("{prefix}.");
            let builds: Vec<&str> = xml.split("<version>").skip(1)
                .filter_map(|s| s.split_once("</version>").map(|v| v.0))
                .filter(|v| v.starts_with(&prefix)).collect();
            let build = builds.last().ok_or_else(|| format!("NeoForge has no release for Minecraft {version}"))?;
            let url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{build}/neoforge-{build}-installer.jar");
            log(format!("Installing NeoForge {build}"));
            download(&url, &dir.join("installer.jar"), None)?;
            run_installer(dir, log)?;
            let args = find_args(dir, "net/neoforged/neoforge", build)
                .ok_or("NeoForge did not create the required launch arguments.")?;
            Ok((Launch::Argfile(args), Some((*build).into())))
        }
    }
}
fn run_installer(dir: &Path, log: &dyn Fn(String)) -> Result<(), String> {
    let status = crate::process::command("java").current_dir(dir)
        .args(["-jar", "installer.jar", "--installServer"])
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().map_err(|e| format!("Java is required to install this server: {e}"))?;
    let report = format!("{}\n{}", String::from_utf8_lossy(&status.stdout), String::from_utf8_lossy(&status.stderr));
    if !status.status.success() {
        return Err(format!("Mod loader installation failed: {}", report.chars().take(1500).collect::<String>()));
    }
    log("Mod loader dependencies installed".into());
    Ok(())
}
fn find_args(dir: &Path, path: &str, release: &str) -> Option<String> {
    let base = dir.join("libraries").join(path).join(release);
    let platform = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
    let file = base.join(platform);
    if file.is_file() {
        file.strip_prefix(dir).ok().map(|p| p.to_string_lossy().replace('\\', "/"))
    } else {
        None
    }
}
fn properties(spec: &NewServer, port: u16) -> String {
    let name: String = spec.name.chars().map(|c| if ['\n', '\r', '=', '\\'].contains(&c) { ' ' } else { c }).collect();
    let seed = match &spec.world { World::New { seed } => seed.as_str(), _ => "" }
        .chars().filter(|c| !['\n', '\r', '=', '\\'].contains(c)).collect::<String>();
    format!(
        "server-port={port}\nserver-ip=\nmotd={name}\nmax-players={}\ngamemode={}\ndifficulty={}\nlevel-name=world\nlevel-seed={seed}\nonline-mode={}\nwhite-list={}\nenforce-whitelist=false\npvp=true\n",
        spec.max_players.clamp(1, 100),
        spec.game_mode,
        spec.difficulty,
        spec.check_accounts,
        spec.friends_only
    )
}
/// Resolve real Minecraft accounts before enabling the whitelist, so server
/// owners aren't accidentally locked out of the world they have just created.
fn provision_players(dir: &Path, spec: &NewServer) -> Result<(), String> {
    if !spec.friends_only {
        return Ok(());
    }
    if spec.players.is_empty() {
        return Err("A friends-only server needs at least one verified Minecraft player. Add your account first, or choose Anyone with the address.".into());
    }
    if !spec.check_accounts {
        return Err("Turn on Minecraft account verification when creating a friends-only Java server.".into());
    }
    let mut allowed = Vec::new();
    for name in &spec.players {
        if !(3..=16).contains(&name.len())
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("Invalid Minecraft account name: {name}"));
        }
        let data = fetch_json(&format!(
            "https://api.minecraftservices.com/minecraft/profile/lookup/name/{name}"
        ))?;
        let uuid = data["id"].as_str().ok_or_else(|| format!(
            "Minecraft account {name} could not be verified for the whitelist."
        ))?;
        if uuid.len() != 32 || !uuid.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid UUID returned for {name}."));
        }
        let formatted = format!("{}-{}-{}-{}-{}",
            &uuid[0..8], &uuid[8..12], &uuid[12..16], &uuid[16..20], &uuid[20..32]);
        allowed.push(json!({ "uuid": formatted, "name": data["name"].as_str().unwrap_or(name) }));
    }
    fs::write(dir.join("whitelist.json"),
        serde_json::to_vec_pretty(&allowed).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if let Some(first) = allowed.first() {
        fs::write(dir.join("ops.json"),
            serde_json::to_vec_pretty(&vec![json!({
                "uuid": first["uuid"], "name": first["name"], "level": 4, "bypassesPlayerLimit": false
            })]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn create(spec: &NewServer, software: Software, version: &str, ram_gb: u8, addons: super::addons::Addons, log: &dyn Fn(String)) -> Result<Entry, Problem> {
    if !spec.eula {
        return Err(Problem::plain("You must accept the Minecraft EULA to create a server."));
    }
    if !(1..=16).contains(&ram_gb) {
        return Err(Problem::plain("Server RAM must be between 1 and 16 GB."));
    }
    crate::process::command("java").arg("-version").output()
        .map_err(|_| Problem::plain("Install a compatible Java runtime and add java to PATH before creating a server."))?;
    let name = if spec.name.trim().is_empty() { "My server" } else { spec.name.trim() };
    let dir = super::new_folder(name);
    fs::create_dir_all(&dir).map_err(|e| Problem::plain(e.to_string()))?;
    let result = (|| -> Result<Entry, String> {
        let port = super::free_port(25565, &super::ports_taken());
        let (launch, loader_version) = install(software, version, &dir, log)?;
        super::addons::install(&dir, software, version, addons, log)?;
        fs::write(dir.join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;
        fs::write(dir.join("server.properties"), properties(spec, port)).map_err(|e| e.to_string())?;
        provision_players(&dir, spec)?;
        if let World::Copy(world) = &spec.world {
            log("Copying your existing world".into());
            super::copy_world(world, &dir.join("world")).map_err(|e| e.to_string())?;
        }
        let mut udp_taken = super::ports_taken();
        udp_taken.push(port);
        let voice_port = if addons.voice_chat {
            let selected = super::free_udp_port(24454, &udp_taken);
            udp_taken.push(selected);
            selected
        } else { 0 };
        let geyser_port = if addons.geyser {
            super::free_udp_port(19132, &udp_taken)
        } else { 0 };
        let meta = Metadata {
            software,
            minecraft_version: version.into(),
            loader_version,
            ram_gb,
            port,
            voice_chat: addons.voice_chat,
            geyser: addons.geyser,
            voice_port,
            geyser_port,
            launch,
        };
        fs::write(dir.join(META), serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let entry = Entry { name: name.into(), dir: dir.clone(), host: None };
        super::add(entry.clone())?;
        Ok(entry)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&dir);
    }
    result.map_err(Problem::plain)
}
/// Apply the creation-dialog options to an existing *stopped* Java server.
/// Worlds, mod folders, plugins not owned by Valtic, and backup files stay put.
/// Changing server software or Minecraft version requires a separate server:
/// rewriting an existing world in place is not a safe migration.
pub fn edit_existing(
    dir: &Path,
    spec: &NewServer,
    software: Software,
    minecraft_version: &str,
    ram_gb: u8,
    addons: super::addons::Addons,
    log: &dyn Fn(String),
) -> Result<(), Problem> {
    let mut meta = metadata(dir).map_err(Problem::plain)?;
    if is_running(dir) {
        return Err(Problem::plain("Stop the server before changing its setup."));
    }
    if meta.software != software || meta.minecraft_version != minecraft_version {
        return Err(Problem::plain(
            "Changing Minecraft version or server software needs a new server. Your existing worlds and plugins have not been changed.",
        ));
    }
    if !(1..=16).contains(&ram_gb) || !(1..=100).contains(&spec.max_players) {
        return Err(Problem::plain("RAM must be 1–16 GB and player limit must be 1–100."));
    }
    if spec.friends_only && !spec.check_accounts {
        return Err(Problem::plain("Friends-only access needs Minecraft account verification enabled."));
    }
    if spec.friends_only && spec.players.is_empty() {
        return Err(Problem::plain("Add at least one allowed Minecraft player before enabling friends-only access."));
    }
    if addons.voice_chat && !super::addons::supports_voice_chat(software) {
        return Err(Problem::plain("This software does not support Simple Voice Chat."));
    }
    if addons.geyser && !super::addons::supports_geyser(software) {
        return Err(Problem::plain("One-click Geyser and Floodgate currently require Paper."));
    }
    if spec.name.contains('\n') || spec.name.contains('\r') || spec.name.len() > 100 {
        return Err(Problem::plain("Use a server name under 100 characters without line breaks."));
    }

    // A settings-only edit should work without an Internet connection.
    // Verify whitelist accounts only when their list actually changes.
    // Never overwrite existing operators when someone edits who can join.
    let before = super::names(dir, "whitelist.json");
    let players_changed = spec.friends_only &&
        (before.len() != spec.players.len()
            || spec.players.iter().any(|name|
                !before.iter().any(|old| old.eq_ignore_ascii_case(name))));
    let verified = dir.join(".valtic-edit-players");
    if players_changed {
        fs::create_dir_all(&verified).map_err(|e| Problem::plain(e.to_string()))?;
        let validated = provision_players(&verified, spec);
        if let Err(err) = validated {
            let _ = fs::remove_dir_all(&verified);
            return Err(Problem::plain(err));
        }
    }

    // Download add-ons only when enabling them or repairing a missing file.
    // Existing working plugin jars must not force a new Modrinth request.
    let addon_dir = if software == Software::Paper { "plugins" } else { "mods" };
    let missing_voice = !dir.join(addon_dir).join("simple-voice-chat.jar").is_file();
    let missing_geyser = !dir.join("plugins/Geyser-Spigot.jar").is_file()
        || !dir.join("plugins/floodgate-spigot.jar").is_file();
    let required = super::addons::Addons {
        voice_chat: addons.voice_chat && (!meta.voice_chat || missing_voice),
        geyser: addons.geyser && (!meta.geyser || missing_geyser),
    };
    let result = super::addons::install(dir, software, minecraft_version, required, log);
    if let Err(err) = result {
        let _ = fs::remove_dir_all(&verified);
        return Err(Problem::plain(err));
    }

    let changes = [
        ("server.motd", spec.name.clone()),
        ("server.max_players", spec.max_players.to_string()),
        ("world.game_mode", spec.game_mode.clone()),
        ("world.difficulty", spec.difficulty.clone()),
        ("server.online_mode", spec.check_accounts.to_string()),
        ("server.whitelist", spec.friends_only.to_string()),
    ];
    set_config(dir, &changes)?;
    if players_changed {
        fs::copy(verified.join("whitelist.json"), dir.join("whitelist.json"))
            .map_err(|e| Problem::plain(format!("Could not update player list: {e}")))?;
    }
    let _ = fs::remove_dir_all(verified);

    // Only Valtic-installed add-ons have fixed filenames. Other plugins and
    // mod jars are never deleted by the settings editor.
    if !addons.voice_chat {
        let path = if software == Software::Paper {
            dir.join("plugins/simple-voice-chat.jar")
        } else {
            dir.join("mods/simple-voice-chat.jar")
        };
        if path.is_file() { fs::remove_file(path).map_err(|e| Problem::plain(e.to_string()))?; }
    }
    if !addons.geyser && software == Software::Paper {
        for name in ["Geyser-Spigot.jar", "floodgate-spigot.jar"] {
            let file = dir.join("plugins").join(name);
            if file.is_file() { fs::remove_file(file).map_err(|e| Problem::plain(e.to_string()))?; }
        }
    }
    if addons.voice_chat && meta.voice_port == 0 {
        let mut used = super::ports_taken();
        used.push(meta.port);
        meta.voice_port = super::free_udp_port(24454, &used);
    }
    if addons.geyser && meta.geyser_port == 0 {
        let mut used = super::ports_taken();
        used.push(meta.port);
        if meta.voice_port != 0 { used.push(meta.voice_port); }
        meta.geyser_port = super::free_udp_port(19132, &used);
    }
    meta.ram_gb = ram_gb;
    meta.voice_chat = addons.voice_chat;
    meta.geyser = addons.geyser;
    fs::write(dir.join(META),
        serde_json::to_vec_pretty(&meta).map_err(|e| Problem::plain(e.to_string()))?)
        .map_err(|e| Problem::plain(e.to_string()))?;
    log("Saved Java server setup. Your world, other plugins and backups were preserved.".into());
    Ok(())
}

/// Install or repair Simple Voice Chat on an existing Paper server. The server
/// must be stopped so the plugin files and metadata are consistent at startup.
/// This is idempotent: pressing Setup again repairs an interrupted installation.
pub fn enable_voice_chat(dir: &Path, log: &dyn Fn(String)) -> Result<(), Problem> {
    let mut meta = metadata(dir).map_err(Problem::plain)?;
    if meta.software != Software::Paper {
        return Err(Problem::plain("One-click voice chat repair currently supports Paper servers."));
    }
    if is_running(dir) {
        return Err(Problem::plain("Stop the Paper server before installing voice chat."));
    }
    super::addons::install(
        dir, meta.software, &meta.minecraft_version,
        super::addons::Addons { voice_chat: true, geyser: false }, log
    ).map_err(Problem::plain)?;
    if meta.voice_port == 0 {
        let mut taken = super::ports_taken();
        taken.push(meta.port);
        meta.voice_port = super::free_udp_port(24454, &taken);
    }
    meta.voice_chat = true;
    fs::write(dir.join(META), serde_json::to_vec_pretty(&meta)
        .map_err(|e| Problem::plain(e.to_string()))?)
        .map_err(|e| Problem::plain(e.to_string()))?;
    log(format!("Simple Voice Chat installed and configured for UDP port {}.", meta.voice_port));
    Ok(())
}

fn launch_args(meta: &Metadata) -> Vec<String> {
    let mut args = vec!["-Xms1G".to_string(), format!("-Xmx{}G", meta.ram_gb)];
    match &meta.launch {
        Launch::Jar(file) => args.extend(["-jar".into(), file.clone(), "nogui".into()]),
        Launch::Argfile(file) => args.extend([format!("@{file}"), "nogui".into()]),
    }
    args
}
pub fn start(dir: &Path) -> Result<String, Problem> {
    let meta = metadata(dir).map_err(Problem::plain)?;
    let mut active = RUNNING.lock().map_err(|_| Problem::plain("Server state is locked."))?;
    if let Some(old) = active.get_mut(dir) {
        if old.child.try_wait().map_err(|e| Problem::plain(e.to_string()))?.is_none() {
            return Err(Problem::plain("This server is already running."));
        }
    }
    active.remove(dir);
    let log_dir = dir.join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| Problem::plain(e.to_string()))?;
    let file = OpenOptions::new().create(true).append(true).open(log_dir.join("valtic-console.log"))
        .map_err(|e| Problem::plain(e.to_string()))?;
    let err = file.try_clone().map_err(|e| Problem::plain(e.to_string()))?;
    let child = crate::process::command("java").current_dir(dir).args(launch_args(&meta))
        .stdin(Stdio::piped()).stdout(Stdio::from(file)).stderr(Stdio::from(err))
        .spawn().map_err(|e| Problem::plain(format!("Failed to launch Java: {e}")))?;
    active.insert(dir.to_owned(), Running { child, started: Instant::now() });
    Ok(format!("Starting {} {} on port {}", meta.software.label(), meta.minecraft_version, meta.port))
}
/// Move only an inactive, registered Java server folder to the Recycle Bin.
pub fn trash_stopped(
    dir: &Path,
    trash: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let mut active = RUNNING.lock().map_err(|_| "Server state is locked.")?;
    if active.get_mut(dir).is_some_and(|p| p.child.try_wait().ok() == Some(None)) {
        return Err("Stop this Java server before deleting it.".into());
    }
    if dir.symlink_metadata().map_err(|e| e.to_string())?.file_type().is_symlink()
        || !dir.join(META).is_file()
    {
        return Err("Not a valid Java server folder.".into());
    }
    trash(dir)?;
    active.remove(dir);
    Ok(())
}
pub fn is_running(dir: &Path) -> bool {
    let Ok(mut active) = RUNNING.lock() else { return false; };
    active.get_mut(dir).is_some_and(|s| s.child.try_wait().ok() == Some(None))
}
pub fn status(dir: &Path) -> Status {
    let mut result = Status::stopped();
    if let Ok(m) = metadata(dir) {
        result.max_players = fs::read_to_string(dir.join("server.properties")).ok()
            .and_then(|s| s.lines().find_map(|l| l.strip_prefix("max-players=")?.parse().ok()));
        result.release = Some(format!("{} {}", m.software.label(), m.minecraft_version));
    }
    if let Ok(mut active) = RUNNING.lock() {
        if let Some(p) = active.get_mut(dir) {
            if p.child.try_wait().ok() == Some(None) {
                result.phase = "running".into();
                result.uptime = Some(p.started.elapsed().as_secs());
            } else {
                active.remove(dir);
            }
        }
    }
    result
}
pub fn command(dir: &Path, line: &str) -> Result<(), String> {
    if line.contains('\r') || line.contains('\n') || line.len() > 1000 {
        return Err("Only a single server command is allowed.".into());
    }
    let mut active = RUNNING.lock().map_err(|e| e.to_string())?;
    let child = active.get_mut(dir).ok_or("The server is not running.")?;
    if child.child.try_wait().map_err(|e| e.to_string())?.is_some() {
        return Err("The Java server has exited.".into());
    }
    let stdin = child.child.stdin.as_mut().ok_or("Server console is unavailable.")?;
    writeln!(stdin, "{line}").and_then(|_| stdin.flush()).map_err(|e| e.to_string())
}
pub fn stop(dir: &Path) -> Result<String, Problem> {
    command(dir, "stop").map_err(Problem::plain)?;
    Ok("Graceful server stop requested.".into())
}
pub fn logs(dir: &Path, since: u64) -> Option<(u64, Vec<String>)> {
    let mut file = File::open(dir.join("logs/valtic-console.log")).ok()?;
    let size = file.metadata().ok()?.len();
    let offset = if since > size { 0 } else { since };
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut data = String::new();
    file.take(256 * 1024).read_to_string(&mut data).ok()?;
    let complete = data.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let lines = data[..complete].lines().map(str::to_owned).collect();
    Some((offset + complete as u64, lines))
}
pub fn config(dir: &Path) -> Option<Value> {
    let p = fs::read_to_string(dir.join("server.properties")).ok()?;
    let get = |k: &str| -> String {
        p.lines().filter_map(|l| l.split_once('=')).find(|(key, _)| *key == k)
            .map(|(_, v)| v.to_string()).unwrap_or_default()
    };
    Some(json!({
        "server": { "port": get("server-port").parse::<u16>().unwrap_or(25565),
                    "max_players": get("max-players").parse::<u32>().unwrap_or(10),
                    "motd": get("motd"),
                    "online_mode": get("online-mode") == "true",
                    "whitelist": get("white-list") == "true" },
        "world": { "name": get("level-name"), "game_mode": get("gamemode"),
                   "difficulty": get("difficulty") },
        "panel": { "bind": "" }
    }))
}
pub fn set_config(dir: &Path, changes: &[(&str, String)]) -> Result<super::Applied, Problem> {
    if is_running(dir) {
        return Err(Problem::plain("Stop this Java server to change its settings, then restart it."));
    }
    let path = dir.join("server.properties");
    let mut lines: Vec<String> = fs::read_to_string(&path)
        .map_err(|e| Problem::plain(e.to_string()))?
        .lines().map(str::to_owned).collect();
    for (key, value) in changes {
        let mapped = match *key {
            "server.port" => "server-port",
            "server.max_players" => "max-players",
            "server.motd" => "motd",
            "server.online_mode" => "online-mode",
            "server.whitelist" => "white-list",
            "world.game_mode" => "gamemode",
            "world.difficulty" => "difficulty",
            "world.name" => "level-name",
            _ => return Err(Problem::plain(format!("Unsupported Java server setting: {key}"))),
        };
        if value.contains('\n') || value.contains('\r') || value.len() > 500 {
            return Err(Problem::plain("Setting value contains unsupported characters."));
        }
        if mapped == "server-port" && value.parse::<u16>().ok().filter(|p| *p > 0).is_none() {
            return Err(Problem::plain("Port must be between 1 and 65535."));
        }
        if mapped == "level-name" && (value.contains('/') || value.contains('\\') || value.contains("..")) {
            return Err(Problem::plain("World folder names cannot contain directory separators or .."));
        }
        if let Some(line) = lines.iter_mut().find(|l| l.starts_with(&format!("{mapped}="))) {
            *line = format!("{mapped}={value}");
        } else {
            lines.push(format!("{mapped}={value}"));
        }
    }
    let content = lines.join("\n") + "\n";
    fs::write(path, content).map_err(|e| Problem::plain(e.to_string()))?;
    if let Some((_, port)) = changes.iter().find(|(key, _)| *key == "server.port") {
        if let Ok(mut meta) = metadata(dir) {
            if let Ok(number) = port.parse::<u16>() {
                meta.port = number;
                fs::write(dir.join(META), serde_json::to_vec_pretty(&meta).map_err(|e| Problem::plain(e.to_string()))?)
                    .map_err(|e| Problem::plain(e.to_string()))?;
            }
        }
    }
    Ok(super::Applied { applied: changes.iter().map(|(k, _)| (*k).into()).collect(), ..Default::default() })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_loader_names_and_safe_launch_args() {
        assert_eq!(Software::parse("neoforge"), Some(Software::Neoforge));
        assert_eq!(Software::parse("unknown"), None);
        let m = Metadata {
            software: Software::Fabric, minecraft_version: "1.21.1".into(),
            loader_version: None, ram_gb: 4, port: 25565,
            voice_chat: false, geyser: false, voice_port: 0, geyser_port: 0,
            launch: Launch::Jar("server.jar".into()),
        };
        assert_eq!(launch_args(&m), ["-Xms1G", "-Xmx4G", "-jar", "server.jar", "nogui"]);
    }
    #[test]
    fn editing_existing_paper_server_keeps_the_world_and_changes_its_settings() {
        let dir = std::env::temp_dir().join(format!(
            "valtic-java-edit-test-{}", std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("world")).unwrap();
        fs::write(dir.join("world/level.dat"), b"existing world").unwrap();
        let meta = Metadata {
            software: Software::Paper,
            minecraft_version: "26.3".into(),
            loader_version: None, ram_gb: 4, port: 25565,
            voice_chat: false, geyser: false, voice_port: 0, geyser_port: 0,
            launch: Launch::Jar("server.jar".into()),
        };
        fs::write(dir.join(META), serde_json::to_vec(&meta).unwrap()).unwrap();
        fs::write(dir.join("server.properties"),
            "server-port=25565\nmax-players=10\ngamemode=survival\ndifficulty=normal\nonline-mode=true\nwhite-list=false\n").unwrap();
        let spec = NewServer {
            name: "Edited server".into(), max_players: 25,
            game_mode: "creative".into(), difficulty: "hard".into(),
            friends_only: false, check_accounts: true,
            ..NewServer::default()
        };
        let addons = super::super::addons::Addons::default();
        edit_existing(&dir, &spec, Software::Paper, "26.3", 7, addons, &|_| {}).unwrap();
        assert_eq!(fs::read(dir.join("world/level.dat")).unwrap(), b"existing world");
        assert_eq!(metadata(&dir).unwrap().ram_gb, 7);
        let text = fs::read_to_string(dir.join("server.properties")).unwrap();
        assert!(text.contains("max-players=25"));
        assert!(text.contains("gamemode=creative"));
        assert!(text.contains("difficulty=hard"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_marker_protects_legacy_worlds() {
        let dir = std::env::temp_dir().join(format!("valtic-java-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(!is_java(&dir));
        fs::write(dir.join(META), "{}").unwrap();
        assert!(is_java(&dir));
        fs::remove_dir_all(&dir).unwrap();
    }
}
