//! Java Edition server hosting for Valtic. A separate runtime from the legacy native server.
//! Server downloads use official Mojang, loader, Paper, and Arclight release endpoints.
use super::{Entry, NewServer, Problem, Status, World};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

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
    Arclight,
}
impl Software {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Paper => "Paper",
            Self::Fabric => "Fabric",
            Self::Forge => "Forge",
            Self::Neoforge => "NeoForge",
            Self::Arclight => "Arclight",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "vanilla" => Some(Self::Vanilla),
            "paper" => Some(Self::Paper),
            "fabric" => Some(Self::Fabric),
            "forge" => Some(Self::Forge),
            "neoforge" => Some(Self::Neoforge),
            "arclight" => Some(Self::Arclight),
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
    /// Blank means use the Java executable found on PATH.
    #[serde(default)]
    pub java_executable: String,
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
    /// Wall-clock start time, used to avoid replaying the previous run's
    /// Minecraft latest.log when the captured stdout is quiet.
    started_wall: SystemTime,
    /// Prior process and system CPU ticks, used for one-second CPU sampling.
    cpu_sample: Option<(Instant, u64, u64)>,
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
/// Minecraft releases supported by each server implementation. These are fetched from
/// the upstream Minecraft and loader projects, not a hard-coded list.
#[derive(Clone, Debug, Default)]
pub struct VersionCatalog {
    pub vanilla: Vec<String>,
    pub paper: Vec<String>,
    pub fabric: Vec<String>,
    pub forge: Vec<String>,
    pub neoforge: Vec<String>,
    pub arclight: Vec<String>,
}

impl VersionCatalog {
    pub fn for_software(&self, software: &str) -> &[String] {
        match software {
            "vanilla" => &self.vanilla,
            "paper" => &self.paper,
            "fabric" => &self.fabric,
            "forge" => &self.forge,
            "neoforge" => &self.neoforge,
            "arclight" => &self.arclight,
            _ => &[],
        }
    }
}

/// Extract all published Maven coordinates without limiting the number of builds.
fn maven_versions(xml: &str) -> Vec<&str> {
    xml.split("<version>").skip(1)
        .filter_map(|s| s.split_once("</version>").map(|(version, _)| version.trim()))
        .collect()
}

fn neoforge_prefix(minecraft: &str) -> Option<String> {
    let parts: Vec<&str> = minecraft.split('.').collect();
    if parts.iter().any(|p| p.parse::<u32>().is_err()) {
        return None;
    }
    match parts.as_slice() {
        ["1", minor] => Some(format!("{minor}.0.")),
        ["1", minor, patch] => Some(format!("{minor}.{patch}.")),
        [year, release] => Some(format!("{year}.{release}.0.")),
        [year, release, patch] => Some(format!("{year}.{release}.{patch}.")),
        _ => None,
    }
}

fn build_order(build: &str) -> Vec<u32> {
    build.split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

fn latest_build<'a>(builds: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let builds: Vec<&str> = builds.into_iter().collect();
    // Prefer actual releases over betas; use betas only if no release exists.
    let stable = builds.iter().copied()
        .filter(|build| !build.contains("-beta") && !build.contains("-alpha"))
        .max_by_key(|build| build_order(build));
    stable.or_else(|| builds.into_iter().max_by_key(|build| build_order(build)))
}


const ARCLIGHT_RELEASES: &str = "https://api.github.com/repos/IzzelAliz/Arclight/releases";

/// Read official release assets rather than guessing a download URL or offering
/// Minecraft versions for which Arclight did not publish a NeoForge server jar.
fn arclight_releases() -> Result<Vec<Value>, String> {
    let mut releases = Vec::new();
    for page in 1..=3 {
        let url = format!("{ARCLIGHT_RELEASES}?per_page=100&page={page}");
        let result = fetch_json(&url)?;
        let page_releases = result.as_array().ok_or("Invalid Arclight releases response")?;
        releases.extend(page_releases.iter().cloned());
        if page_releases.len() < 100 { break; }
    }
    Ok(releases)
}

fn arclight_asset<'a>(releases: &'a [Value], version: &str) -> Option<&'a Value> {
    let prefix = format!("arclight-neoforge-{version}-");
    let mut fallback = None;
    for release in releases {
        let Some(assets) = release["assets"].as_array() else { continue };
        for asset in assets {
            let Some(name) = asset["name"].as_str() else { continue };
            if !name.starts_with(&prefix) || !name.ends_with(".jar") { continue; }
            if fallback.is_none() { fallback = Some(asset); }
            // Prefer a stable build; use an official snapshot only if necessary.
            if release["prerelease"] != true && !name.contains("SNAPSHOT") {
                return Some(asset);
            }
        }
    }
    fallback
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    use sha2::{Digest as _, Sha256};
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hash.update(&buf[..n]);
    }
    if format!("{:x}", hash.finalize()) != expected.to_ascii_lowercase() {
        return Err("Arclight release checksum mismatch. Download cancelled.".into());
    }
    Ok(())
}

pub fn versions() -> Result<VersionCatalog, String> {
    let manifest = fetch_json(MANIFEST)?;
    let releases: Vec<String> = manifest["versions"].as_array()
        .ok_or("Invalid Mojang version manifest")?
        .iter().filter(|item| item["type"] == "release")
        .filter_map(|item| item["id"].as_str().map(str::to_owned))
        .collect();
    if releases.is_empty() {
        return Err("Mojang returned no Minecraft releases".into());
    }

    // Fabric publishes its own supported game list. Matching it to Mojang's
    // releases keeps snapshots out of the normal Java server picker.
    let fabric_games: HashSet<String> = fetch_json("https://meta.fabricmc.net/v2/versions/game")
        .ok().and_then(|data| data.as_array().map(|items| items.iter()
            .filter_map(|item| item["version"].as_str().map(str::to_owned))
            .collect())).unwrap_or_default();
    let forge_builds = fetch_text(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml"
    ).map(|xml| maven_versions(&xml).iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        .unwrap_or_default();
    let neo_builds = fetch_text(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml"
    ).map(|xml| maven_versions(&xml).iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        .unwrap_or_default();
    // NeoForge 1.20.1 used a differently named Maven artifact.
    let old_neo_builds = fetch_text(
        "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml"
    ).map(|xml| maven_versions(&xml).iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        .unwrap_or_default();

    let arclight = arclight_releases().unwrap_or_default();
    Ok(VersionCatalog {
        vanilla: releases.clone(),
        // Paper availability is validated against Paper's actual stable builds
        // when installing; do not silently replace a selected version.
        paper: releases.clone(),
        fabric: releases.iter().filter(|v| fabric_games.contains(*v)).cloned().collect(),
        forge: releases.iter().filter(|v| {
            let prefix = format!("{v}-");
            forge_builds.iter().any(|build| build.starts_with(&prefix))
        }).cloned().collect(),
        neoforge: releases.iter().filter(|v| {
            if v.as_str() == "1.20.1" {
                old_neo_builds.iter().any(|build| build.starts_with("1.20.1-"))
            } else {
                neoforge_prefix(v).is_some_and(|prefix|
                    neo_builds.iter().any(|build| build.starts_with(&prefix)))
            }
        }).cloned().collect(),
        arclight: releases.iter().filter(|v| arclight_asset(&arclight, v).is_some())
            .cloned().collect(),
    })
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
fn install(software: Software, version: &str, dir: &Path, java_executable: &str, log: &dyn Fn(String)) -> Result<(Launch, Option<String>), String> {
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
            let xml = fetch_text("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")?;
            let prefix = format!("{version}-");
            let published: Vec<&str> = maven_versions(&xml).into_iter()
                .filter_map(|build| build.strip_prefix(&prefix)).collect();
            let promos = fetch_json("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json").ok();
            let recommended = promos.as_ref().and_then(|data|
                data["promos"][format!("{version}-recommended")].as_str()
                .or_else(|| data["promos"][format!("{version}-latest")].as_str()));
            let build = recommended.filter(|build| published.contains(build))
                .or_else(|| latest_build(published.iter().copied()))
                .ok_or_else(|| format!("Forge has no published build for Minecraft {version}"))?;
            let release = format!("{version}-{build}");
            let url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{release}/forge-{release}-installer.jar");
            log(format!("Installing Forge {release}"));
            download(&url, &dir.join("installer.jar"), None)?;
            run_installer(dir, java_executable, log)?;
            let launch = find_args(dir, "net/minecraftforge/forge", &release)
                .map(Launch::Argfile)
                .or_else(|| find_legacy_forge_jar(dir, &release).map(Launch::Jar))
                .ok_or("Forge did not produce server launch arguments or a legacy server jar.")?;
            Ok((launch, Some(build.into())))
        }
        Software::Neoforge => {
            let old = version == "1.20.1";
            let (artifact, prefix) = if old {
                ("forge", "1.20.1-".to_owned())
            } else {
                ("neoforge", neoforge_prefix(version)
                    .ok_or_else(|| format!("NeoForge does not support version {version}"))?)
            };
            let group = format!("net/neoforged/{artifact}");
            let xml = fetch_text(&format!(
                "https://maven.neoforged.net/releases/{group}/maven-metadata.xml"
            ))?;
            let build = latest_build(maven_versions(&xml).into_iter()
                .filter(|build| build.starts_with(&prefix)))
                .ok_or_else(|| format!("NeoForge has no published build for Minecraft {version}"))?;
            let url = format!("https://maven.neoforged.net/releases/{group}/{build}/{artifact}-{build}-installer.jar");
            log(format!("Installing NeoForge {build}"));
            download(&url, &dir.join("installer.jar"), None)?;
            run_installer(dir, java_executable, log)?;
            let args = find_args(dir, &group, build)
                .ok_or("NeoForge did not create the required launch arguments.")?;
            Ok((Launch::Argfile(args), Some(build.into())))
        }
        Software::Arclight => {
            let releases = arclight_releases()?;
            let asset = arclight_asset(&releases, version)
                .ok_or_else(|| format!("Arclight has no NeoForge build for Minecraft {version}"))?;
            let url = asset["browser_download_url"].as_str()
                .ok_or("Arclight release is missing its download URL")?;
            if !url.starts_with("https://github.com/IzzelAliz/Arclight/releases/download/")
                || !url.ends_with(".jar")
            {
                return Err("Unexpected Arclight download URL.".into());
            }
            let filename = asset["name"].as_str().ok_or("Arclight asset name is missing")?;
            let build = filename.strip_prefix(&format!("arclight-neoforge-{version}-"))
                .and_then(|s| s.strip_suffix(".jar"))
                .ok_or("Unexpected Arclight asset filename")?;
            log(format!("Downloading official Arclight NeoForge {build} for Minecraft {version}"));
            let jar = dir.join("server.jar");
            download(url, &jar, None)?;
            if let Some(digest) = asset["digest"].as_str() {
                let checksum = digest.strip_prefix("sha256:")
                    .filter(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or("Invalid Arclight release checksum")?;
                if let Err(error) = verify_sha256(&jar, checksum) {
                    let _ = fs::remove_file(jar);
                    return Err(error);
                }
            }
            Ok((Launch::Jar("server.jar".into()), Some(build.into())))
        }
    }
}
fn run_installer(dir: &Path, java_executable: &str, log: &dyn Fn(String)) -> Result<(), String> {
    let status = crate::process::command(java_program(java_executable)).current_dir(dir)
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
/// Before Forge 1.17, installers could create a runnable server jar rather
/// than the modern JVM argument files.
fn find_legacy_forge_jar(dir: &Path, release: &str) -> Option<String> {
    for name in [format!("forge-{release}.jar"), format!("forge-{release}-universal.jar")] {
        if dir.join(&name).is_file() {
            return Some(name);
        }
    }
    None
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
/// Create using system Java for callers that do not choose a runtime.
pub fn create(spec: &NewServer, software: Software, version: &str, ram_gb: u8, addons: super::addons::Addons, log: &dyn Fn(String)) -> Result<Entry, Problem> {
    create_with_java(spec, software, version, ram_gb, addons, "", log)
}

/// Use the selected executable for installation as well as subsequent server starts.
pub fn create_with_java(spec: &NewServer, software: Software, version: &str, ram_gb: u8, addons: super::addons::Addons, java_executable: &str, log: &dyn Fn(String)) -> Result<Entry, Problem> {
    if !spec.eula {
        return Err(Problem::plain("You must accept the Minecraft EULA to create a server."));
    }
    if !(1..=16).contains(&ram_gb) {
        return Err(Problem::plain("Server RAM must be between 1 and 16 GB."));
    }
    validate_java_executable(java_executable)?;
    let check = crate::process::command(java_program(java_executable)).arg("-version").output()
        .map_err(|e| Problem::plain(format!("Cannot start the selected Java runtime: {e}")))?;
    if !check.status.success() {
        return Err(Problem::plain("The selected Java runtime did not start successfully."));
    }
    let name = if spec.name.trim().is_empty() { "My server" } else { spec.name.trim() };
    let dir = super::new_folder(name);
    fs::create_dir_all(&dir).map_err(|e| Problem::plain(e.to_string()))?;
    let result = (|| -> Result<Entry, String> {
        let port = super::free_port(25565, &super::ports_taken());
        let (launch, loader_version) = install(software, version, &dir, java_executable, log)?;
        if software == Software::Arclight {
            fs::create_dir_all(dir.join("mods")).map_err(|e| e.to_string())?;
            fs::create_dir_all(dir.join("plugins")).map_err(|e| e.to_string())?;
        }
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
            java_executable: java_executable.trim().trim_matches('"').to_string(),
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


pub fn ensure_voice_chat(dir: &Path, log: &dyn Fn(String)) -> Result<(), Problem> {
    let mut meta = metadata(dir).map_err(Problem::plain)?;
    if !meta.voice_chat { return Ok(()); }
    if is_running(dir) {
        return Err(Problem::plain("Stop the Java server before repairing voice chat."));
    }
    let plugin = if meta.software == Software::Paper {
        dir.join("plugins/simple-voice-chat.jar")
    } else {
        dir.join("mods/simple-voice-chat.jar")
    };
    if !plugin.is_file() || fs::metadata(&plugin).is_ok_and(|m| m.len() == 0) {
        log("Voice chat is enabled but its plugin is missing; installing it automatically.".into());
        super::addons::install(
            dir, meta.software, &meta.minecraft_version,
            super::addons::Addons { voice_chat: true, geyser: false }, log
        ).map_err(Problem::plain)?;
    }
    if meta.voice_port == 0 {
        let mut used = super::ports_taken();
        used.push(meta.port);
        meta.voice_port = super::free_udp_port(24454, &used);
        fs::write(dir.join(META),
            serde_json::to_vec_pretty(&meta).map_err(|e| Problem::plain(e.to_string()))?)
            .map_err(|e| Problem::plain(e.to_string()))?;
    }
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
/// Discover local Java installations for the server setup picker. This only
/// walks installation folders and PATH; it never changes system settings.
/// Each executable is verified when the player selects it.
pub fn detected_java_runtimes() -> Vec<(String, PathBuf)> {
    let executable = if cfg!(windows) { "java.exe" } else { "java" };
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |path: PathBuf| {
        if !path.is_file() || !seen.insert(path.to_string_lossy().to_lowercase()) {
            return;
        }
        let home = path.parent().and_then(Path::parent);
        let folder = home.and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Java".to_string());
        let label = home.and_then(crate::java::release_version)
            .map(|(version, major)| format!("Java {major} ({version}) · {folder}"))
            .unwrap_or(folder);
        results.push((label, path));
    };
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        add(PathBuf::from(home).join("bin").join(executable));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            add(directory.join(executable));
        }
    }
    if cfg!(windows) {
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Some(root) = std::env::var_os(variable) else { continue };
            for vendor in ["Eclipse Adoptium", "Java", "Microsoft", "Zulu", "Amazon Corretto", "BellSoft"] {
                if let Ok(installs) = fs::read_dir(PathBuf::from(&root).join(vendor)) {
                    for install in installs.flatten() {
                        add(install.path().join("bin").join(executable));
                    }
                }
            }
        }
    } else if let Ok(installs) = fs::read_dir("/usr/lib/jvm") {
        for install in installs.flatten() {
            add(install.path().join("bin").join(executable));
        }
    }
    // Also offer Minecraft's own installed runtimes, not just system JDKs.
    for runtime in crate::java::mojang_runtimes(&crate::java::runtimes_dir()) {
        add(runtime.home.join("bin").join(executable));
    }
    results
}

fn java_program(value: &str) -> &str {
    let selected = value.trim().trim_matches('"');
    if selected.is_empty() { "java" } else { selected }
}

/// Validate a per-server runtime without modifying the machine-wide Java selection.
pub fn validate_java_executable(value: &str) -> Result<(), Problem> {
    let path = value.trim().trim_matches('"');
    if path.is_empty() {
        return Ok(()); // Use java from PATH
    }
    let executable = Path::new(path);
    if !executable.is_file() {
        return Err(Problem::plain(format!("Java executable not found: {path}. Select the java.exe file inside your JDK's bin folder.")));
    }
    let output = crate::process::command(path).arg("-version").output()
        .map_err(|e| Problem::plain(format!("Could not run selected Java: {e}")))?;
    if !output.status.success() {
        return Err(Problem::plain("The selected Java executable did not start successfully."));
    }
    Ok(())
}

/// Set the Java runtime for an existing stopped server and persist it to metadata.
pub fn set_java_executable(dir: &Path, value: &str) -> Result<(), Problem> {
    if is_running(dir) {
        return Err(Problem::plain("Stop the server before changing its Java runtime."));
    }
    validate_java_executable(value)?;
    let mut meta = metadata(dir).map_err(Problem::plain)?;
    meta.java_executable = value.trim().trim_matches('"').to_string();
    fs::write(dir.join(META),
        serde_json::to_vec_pretty(&meta).map_err(|e| Problem::plain(e.to_string()))?)
        .map_err(|e| Problem::plain(e.to_string()))
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
    validate_java_executable(&meta.java_executable)?;
    let java = java_program(&meta.java_executable);
    let child = crate::process::command(java).current_dir(dir).args(launch_args(&meta))
        .stdin(Stdio::piped()).stdout(Stdio::from(file)).stderr(Stdio::from(err))
        .spawn().map_err(|e| Problem::plain(format!("Failed to launch Java: {e}")))?;
    active.insert(dir.to_owned(), Running {
        child,
        started: Instant::now(),
        started_wall: SystemTime::now(),
        cpu_sample: None,
    });
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
/// Samples only the running Minecraft Java process, not the entire PC.
#[cfg(target_os = "windows")]
fn process_usage(running: &mut Running) -> (Option<u64>, Option<f64>) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::GetProcessTimes;

    let handle = running.child.as_raw_handle();
    let mut memory_info: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    memory_info.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let memory = (unsafe {
        GetProcessMemoryInfo(handle, &mut memory_info, memory_info.cb)
    } != 0).then_some(memory_info.WorkingSetSize as u64);

    let mut created: FILETIME = unsafe { std::mem::zeroed() };
    let mut exited: FILETIME = unsafe { std::mem::zeroed() };
    let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
    let mut user: FILETIME = unsafe { std::mem::zeroed() };
    if unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        return (memory, None);
    }

    let to_ticks = |time: FILETIME| {
        ((time.dwHighDateTime as u64) << 32) | (time.dwLowDateTime as u64)
    };
    let ticks = to_ticks(kernel).saturating_add(to_ticks(user));
    let now = Instant::now();
    let cpu = running.cpu_sample.and_then(|(last, previous_ticks, _)| {
        let seconds = now.saturating_duration_since(last).as_secs_f64();
        if seconds <= 0.0 {
            return None;
        }
        let cores = std::thread::available_parallelism().map_or(1, |count| count.get()) as f64;
        // Process CPU time is in 100-nanosecond intervals. Divide by
        // available logical CPUs so 100% means the entire computer.
        Some((ticks.saturating_sub(previous_ticks) as f64
            / (seconds * 10_000_000.0 * cores) * 100.0).clamp(0.0, 100.0))
    });
    running.cpu_sample = Some((now, ticks, 0));
    (memory, cpu)
}

#[cfg(target_os = "linux")]
fn process_usage(running: &mut Running) -> (Option<u64>, Option<f64>) {
    let pid = running.child.id();
    let memory = fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmRSS:")?
                    .split_whitespace().next()?.parse::<u64>().ok()
            })
        })
        .and_then(|kib| kib.checked_mul(1024));
    let process_ticks = fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            // Fields following the final ") " start at field 3; 14 and 15
            // are the process's user and kernel CPU jiffies respectively.
            let (_, fields) = stat.rsplit_once(") ")?;
            let parts: Vec<&str> = fields.split_whitespace().collect();
            Some(parts.get(11)?.parse::<u64>().ok()?
                .saturating_add(parts.get(12)?.parse::<u64>().ok()?))
        });
    let system_ticks = fs::read_to_string("/proc/stat")
        .ok()
        .and_then(|stat| {
            let mut parts = stat.lines().next()?.split_whitespace();
            (parts.next()? == "cpu")
                .then(|| parts.filter_map(|part| part.parse::<u64>().ok()).sum::<u64>())
        });
    let cpu = process_ticks.zip(system_ticks).and_then(|(process, system)| {
        let previous = running.cpu_sample;
        running.cpu_sample = Some((Instant::now(), process, system));
        previous.and_then(|(_, last_process, last_system)| {
            let elapsed = system.saturating_sub(last_system);
            (elapsed > 0).then(|| {
                (100.0 * process.saturating_sub(last_process) as f64 / elapsed as f64)
                    .clamp(0.0, 100.0)
            })
        })
    });
    (memory, cpu)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn process_usage(_running: &mut Running) -> (Option<u64>, Option<f64>) {
    (None, None)
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
                let (ram, cpu) = process_usage(p);
                result.ram_bytes = ram;
                result.cpu_percent = cpu;
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

/// Immediately terminate only the Java child process tracked for this server.
/// Unlike `stop`, this does not give the game a chance to save its world.
pub fn kill(dir: &Path) -> Result<String, Problem> {
    let mut active = RUNNING.lock().map_err(|_| Problem::plain("Server state is locked."))?;
    let running = active.get_mut(dir)
        .ok_or_else(|| Problem::plain("This Java server is not running in this launcher session."))?;
    if running.child.try_wait().map_err(|e| Problem::plain(e.to_string()))?.is_some() {
        active.remove(dir);
        return Err(Problem::plain("The Java server has already exited."));
    }
    running.child.kill().map_err(|e| Problem::plain(format!("Could not kill Java server: {e}")))?;
    running.child.wait().map_err(|e| Problem::plain(format!("Could not reap Java server: {e}")))?;
    active.remove(dir);
    Ok("Java server was killed. Unsaved world changes may be lost.".into())
}
/// Byte offset for the next fresh console session. Java appends to the same
/// file across restarts, so replaying from zero would show old boot messages.
pub fn log_size(dir: &Path) -> u64 {
    fs::metadata(dir.join("logs/valtic-console.log"))
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

// This bit is reserved for the fallback Minecraft log cursor. Console log
// files will never approach 8 EiB, so it cannot conflict with a byte offset.
const MINECRAFT_LOG_CURSOR: u64 = 1 << 63;

/// Read a single complete batch, retaining any unfinished final line for the
/// next poll. A long unbroken line is still consumed in bounded chunks.
fn read_log_chunk(path: &Path, since: u64) -> Option<(u64, Vec<String>)> {
    const MAX_READ: usize = 256 * 1024;
    let mut file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let offset = if since > size { 0 } else { since };
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut data = Vec::new();
    file.take(MAX_READ as u64).read_to_end(&mut data).ok()?;
    let complete = data.iter().rposition(|&byte| byte == b'\n')
        .map(|index| index + 1)
        .unwrap_or_else(|| if data.len() == MAX_READ { data.len() } else { 0 });
    let lines = String::from_utf8_lossy(&data[..complete])
        .lines().map(str::to_owned).collect();
    Some((offset + complete as u64, lines))
}

/// Follow the launcher-captured stdout, falling back to Minecraft's own
/// latest.log if the process writes there but stops emitting on stdout.
/// The fallback only activates when latest.log was actually updated *after*
/// this launch, avoiding the illusion of a server running before Start.
pub fn logs(dir: &Path, since: u64) -> Option<(u64, Vec<String>)> {
    let captured = dir.join("logs/valtic-console.log");
    let latest = dir.join("logs/latest.log");
    if since & MINECRAFT_LOG_CURSOR != 0 {
        return read_log_chunk(&latest, since & !MINECRAFT_LOG_CURSOR)
            .map(|(next, lines)| (next | MINECRAFT_LOG_CURSOR, lines))
            .or_else(|| read_log_chunk(&captured, 0));
    }
    let from_stdout = read_log_chunk(&captured, since);
    if from_stdout.as_ref().is_some_and(|(_, lines)| !lines.is_empty()) {
        return from_stdout;
    }
    let started = RUNNING.lock().ok().and_then(|active| {
        active.get(dir).map(|process| process.started_wall)
    });
    let minecraft_newer = started.is_some_and(|started| {
        let latest_changed = fs::metadata(&latest).and_then(|m| m.modified()).ok();
        let captured_changed = fs::metadata(&captured).and_then(|m| m.modified()).ok();
        latest_changed.is_some_and(|latest_at| {
            latest_at > started && captured_changed.is_none_or(|captured_at| latest_at > captured_at)
        })
    });
    if minecraft_newer {
        if let Some((next, lines)) = read_log_chunk(&latest, 0) {
            return Some((next | MINECRAFT_LOG_CURSOR, lines));
        }
    }
    from_stdout
}

const SETUP_ONLY_PROPERTIES: &[&str] = &[
    "server-port", "server-ip", "level-name", "level-seed",
    "online-mode", "white-list", "enforce-whitelist",
    "enable-rcon", "rcon.password", "rcon.port", "query.port",
];

pub fn property_editable(key: &str) -> bool {
    !SETUP_ONLY_PROPERTIES.contains(&key)
        && !key.is_empty()
        && key.bytes().all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
}

pub fn list_properties(dir: &Path) -> Result<Vec<(String, String)>, String> {
    if !is_java(dir) { return Err("Only Java servers have server.properties.".into()); }
    let content = fs::read_to_string(dir.join("server.properties")).map_err(|e| e.to_string())?;
    Ok(content.lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once('=')?;
            let key = key.trim();
            (!key.is_empty() && !key.starts_with('#'))
                .then(|| (key.to_string(), value.to_string()))
        }).collect())
}

/// Updates exactly one allowlisted-to-edit Java property, preserving comments and
/// other settings. Never changes the world folder, port or account security behind
/// the existing setup/whitelist controls.
pub fn set_property(dir: &Path, key: &str, value: &str) -> Result<(), Problem> {
    if !is_java(dir) { return Err(Problem::plain("Not a Java server.")); }
    if is_running(dir) { return Err(Problem::plain("Stop the server before editing properties.")); }
    if !property_editable(key) { return Err(Problem::plain("Change this option in Edit server setup.")); }
    if value.len() > 500 || value.contains('\n') || value.contains('\r') || value.contains('\0') {
        return Err(Problem::plain("Property values must be one line and at most 500 characters."));
    }
    let path = dir.join("server.properties");
    let old = fs::read_to_string(&path).map_err(|e| Problem::plain(e.to_string()))?;
    let mut found = false;
    let mut lines = Vec::new();
    for line in old.lines() {
        if line.split_once('=').is_some_and(|(name, _)| name.trim() == key) {
            if !found { lines.push(format!("{key}={value}")); found = true; }
        } else { lines.push(line.to_string()); }
    }
    if !found { return Err(Problem::plain("Property no longer exists. Refresh the tab.")); }
    // Windows does not allow fs::rename to replace an existing file.
    // The server is stopped here, so update its existing properties file directly.
    fs::write(&path, lines.join("\n") + "\n").map_err(|e| Problem::plain(e.to_string()))?;
    Ok(())
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
    fn catalog_parsers_cover_older_and_modern_versions() {
        assert_eq!(neoforge_prefix("1.21.1").as_deref(), Some("21.1."));
        assert_eq!(neoforge_prefix("26.3").as_deref(), Some("26.3.0."));
        assert_eq!(neoforge_prefix("1.20.2").as_deref(), Some("20.2."));
        assert_eq!(latest_build(["47.3.0", "47.4.10", "47.4.9"]), Some("47.4.10"));
        assert_eq!(latest_build(["21.1.20-beta", "21.1.10"]), Some("21.1.10"));
        assert_eq!(maven_versions("<versions><version>1.8.9-11.15.1</version><version>26.3-66.1.0</version></versions>"),
            vec!["1.8.9-11.15.1", "26.3-66.1.0"]);
    }
    #[test]
    fn arclight_selects_exact_neoforge_version_and_prefers_stable() {
        let releases = json!([
            {"prerelease": true, "assets": [
                {"name": "arclight-neoforge-1.21.1-1.0.2-SNAPSHOT-abc.jar"},
                {"name": "arclight-neoforge-1.21.11-1.0.0-SNAPSHOT-def.jar"}
            ]},
            {"prerelease": false, "assets": [
                {"name": "arclight-fabric-1.21.1-1.0.1-xyz.jar"},
                {"name": "arclight-neoforge-1.21.1-1.0.1-xyz.jar"}
            ]}
        ]);
        let entries = releases.as_array().unwrap();
        assert_eq!(
            arclight_asset(entries, "1.21.1").unwrap()["name"],
            "arclight-neoforge-1.21.1-1.0.1-xyz.jar"
        );
        assert_eq!(
            arclight_asset(entries, "1.21.11").unwrap()["name"],
            "arclight-neoforge-1.21.11-1.0.0-SNAPSHOT-def.jar"
        );
        assert!(arclight_asset(entries, "1.20.1").is_none());
    }
    #[test]
    fn parses_loader_names_and_safe_launch_args() {
        assert_eq!(Software::parse("neoforge"), Some(Software::Neoforge));
        assert_eq!(Software::parse("arclight"), Some(Software::Arclight));
        assert_eq!(Software::Arclight.label(), "Arclight");
        assert_eq!(Software::parse("unknown"), None);
        let m = Metadata {
            software: Software::Fabric, minecraft_version: "1.21.1".into(),
            loader_version: None, ram_gb: 4, java_executable: String::new(), port: 25565,
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
            loader_version: None, ram_gb: 4, java_executable: String::new(), port: 25565,
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
    fn java_executable_selection_does_not_change_system_java() {
        assert_eq!(java_program(""), "java");
        assert_eq!(java_program("  "), "java");
        assert_eq!(java_program(r#""C:\jdk-21\bin\java.exe""#), r"C:\jdk-21\bin\java.exe");
    }

    #[test]
    fn console_logs_keep_streaming_through_invalid_utf8() {
        let dir = std::env::temp_dir().join(format!(
            "valtic-java-console-{}-{:?}", std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("logs")).unwrap();
        fs::write(dir.join("logs/valtic-console.log"), b"booting\n\xffserver ready\n").unwrap();

        let (next, first) = logs(&dir, 0).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0], "booting");
        assert!(first[1].contains("server ready"));
        fs::write(dir.join("logs/valtic-console.log"), b"restarted\n").unwrap();
        let (_, fresh) = logs(&dir, next).unwrap();
        assert_eq!(fresh, ["restarted"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn console_logs_advance_past_oversized_line() {
        let dir = std::env::temp_dir().join(format!(
            "valtic-java-large-console-{}-{:?}", std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("logs")).unwrap();
        let mut bytes = vec![b'x'; 256 * 1024 + 3];
        bytes.extend_from_slice(b"\nfinished\n");
        fs::write(dir.join("logs/valtic-console.log"), bytes).unwrap();
        let (next, first) = logs(&dir, 0).unwrap();
        assert!(next > 0);
        assert_eq!(first.len(), 1);
        let (_, tail) = logs(&dir, next).unwrap();
        assert_eq!(tail.last().map(String::as_str), Some("finished"));
        let _ = fs::remove_dir_all(dir);
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
