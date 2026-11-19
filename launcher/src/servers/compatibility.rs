//! Offline preflight for Java server mods, using the metadata inside installed JARs.
use super::java::{self, Software};
use super::Problem;
use serde_json::Value as Json;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Default, Debug)]
pub struct Report {
    pub checked: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
impl Report {
    pub fn summary(&self) -> String {
        let mut lines = vec![format!("Checked {} mod JAR(s).", self.checked)];
        if self.errors.is_empty() && self.warnings.is_empty() {
            lines.push("No known incompatibilities detected. This cannot guarantee a modded server will start.".into());
        }
        lines.extend(self.errors.iter().map(|s| format!("INCOMPATIBLE: {s}")));
        lines.extend(self.warnings.iter().map(|s| format!("CHECK: {s}")));
        lines.join("\n")
    }
    pub fn block_start(&self) -> Result<(), Problem> {
        if self.errors.is_empty() { return Ok(()); }
        Err(Problem::plain(format!("Incompatible server mods detected. Startup cancelled.\n{}\nRemove or replace the incompatible JAR, then try again.", self.summary())))
    }
}
#[derive(Debug)]
struct Dependency { owner: String, id: String, requirement: String }
#[derive(Debug)]
struct JarMod {
    filename: String,
    ids: Vec<(String, String)>,
    dependencies: Vec<Dependency>,
}
fn descriptor(zip: &mut ZipArchive<File>, name: &str) -> Option<String> {
    let entry = zip.by_name(name).ok()?;
    if entry.size() > 256 * 1024 { return None; }
    let mut body = String::new();
    entry.take(256 * 1024).read_to_string(&mut body).ok()?;
    Some(body)
}
fn forge_descriptor(input: &str, filename: &str) -> Option<JarMod> {
    let info = input.parse::<toml::Value>().ok()?;
    let ids: Vec<(String, String)> = info.get("mods")?.as_array()?.iter().filter_map(|m| Some((
        m.get("modId")?.as_str()?.to_lowercase(),
        m.get("version").and_then(toml::Value::as_str).unwrap_or("").to_string()
    ))).collect();
    if ids.is_empty() { return None; }
    let mut dependencies = Vec::new();
    if let Some(groups) = info.get("dependencies").and_then(toml::Value::as_table) {
        for (owner, entries) in groups {
            for dep in entries.as_array().into_iter().flatten() {
                let Some(id) = dep.get("modId").and_then(toml::Value::as_str) else { continue };
                let required = dep.get("mandatory").and_then(toml::Value::as_bool).unwrap_or_else(|| {
                    dep.get("type").and_then(toml::Value::as_str).is_some_and(|s| s.eq_ignore_ascii_case("required"))
                });
                let client_only = dep.get("side").and_then(toml::Value::as_str)
                    .is_some_and(|s| s.eq_ignore_ascii_case("client"));
                if required && !client_only {
                    dependencies.push(Dependency {
                        owner: owner.to_owned(),
                        id: id.to_lowercase(),
                        requirement: dep.get("versionRange").and_then(toml::Value::as_str).unwrap_or("*").to_string(),
                    });
                }
            }
        }
    }
    Some(JarMod { filename: filename.into(), ids, dependencies })
}
fn fabric_descriptor(input: &str, filename: &str) -> Option<JarMod> {
    let info: Json = serde_json::from_str(input).ok()?;
    let id = info["id"].as_str()?.to_lowercase();
    let version = info["version"].as_str().unwrap_or("").to_string();
    let dependencies = info["depends"].as_object().into_iter().flatten().filter_map(|(dep_id, req)| {
        let requirement = match req {
            Json::String(s) => s.clone(),
            Json::Array(items) => items.iter().filter_map(Json::as_str).collect::<Vec<_>>().join(" || "),
            _ => return None,
        };
        Some(Dependency { owner: id.clone(), id: dep_id.to_lowercase(), requirement })
    }).collect();
    Some(JarMod { filename: filename.into(), ids: vec![(id, version)], dependencies })
}
fn numbers(v: &str) -> Option<Vec<u32>> {
    let v = v.trim().trim_start_matches('v').split(['+', '-']).next()?;
    let parts: Option<Vec<u32>> = v.split('.').map(|s| s.parse().ok()).collect();
    parts.filter(|v| !v.is_empty())
}
fn compare(actual: &str, needed: &str) -> Option<std::cmp::Ordering> {
    let a = numbers(actual)?;
    let b = numbers(needed)?;
    for index in 0..a.len().max(b.len()) {
        let result = a.get(index).copied().unwrap_or(0).cmp(&b.get(index).copied().unwrap_or(0));
        if result != std::cmp::Ordering::Equal { return Some(result); }
    }
    Some(std::cmp::Ordering::Equal)
}
fn matches_req(actual: &str, req: &str) -> Option<bool> {
    use std::cmp::Ordering;
    let req = req.trim();
    if req.is_empty() || req == "*" {
        return Some(true);
    }
    // Exact Fabric versions must not be interpreted as semver caret ranges.
    if numbers(req).is_some() {
        return compare(actual, req).map(|c| c == Ordering::Equal);
    }
    // NeoForge/Forge singleton range, e.g. [1.21.1].
    if req.starts_with('[') && req.ends_with(']') && !req.contains(',') {
        return compare(actual, &req[1..req.len() - 1]).map(|c| c == Ordering::Equal);
    }
    if (req.starts_with('[') || req.starts_with('('))
        && (req.ends_with(']') || req.ends_with(')')) && req.contains(',')
    {
        let (low, high) = req[1..req.len() - 1].split_once(',')?;
        let lower_ok = if low.trim().is_empty() {
            true
        } else {
            let c = compare(actual, low.trim())?;
            c == Ordering::Greater || (req.starts_with('[') && c == Ordering::Equal)
        };
        let upper_ok = if high.trim().is_empty() {
            true
        } else {
            let c = compare(actual, high.trim())?;
            c == Ordering::Less || (req.ends_with(']') && c == Ordering::Equal)
        };
        return Some(lower_ok && upper_ok);
    }
    let v = semver::Version::parse(actual).ok()?;
    let mut matched = false;
    for alt in req.split("||") {
        let normalized = alt.split_whitespace().collect::<Vec<_>>().join(", ");
        let range = semver::VersionReq::parse(&normalized).ok()?;
        matched = true;
        if range.matches(&v) {
            return Some(true);
        }
    }
    matched.then_some(false)
}
fn neoforge_version(dir: &Path, software: Software, loader: Option<&str>) -> Option<String> {
    if software == Software::Neoforge { return loader.map(str::to_string); }
    if software != Software::Arclight { return None; }
    // Arclight loader_version is an Arclight build ID, not the NeoForge version.
    let root = dir.join("libraries/net/neoforged/neoforge");
    let mut versions = fs::read_dir(root).ok()?.filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|version| numbers(version).is_some())
        .collect::<Vec<_>>();
    versions.sort(); versions.dedup();
    if versions.len() == 1 { versions.pop() } else { None }
}
pub fn check(dir: &Path) -> Result<Report, String> {
    let meta = java::metadata(dir)?;
    let mut report = Report::default();
    if !dir.join("mods").exists() { return Ok(report); }
    let mut files = fs::read_dir(dir.join("mods")).map_err(|e| format!("Cannot read the mods folder: {e}"))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("jar")))
        .collect::<Vec<_>>();
    files.sort();
    report.checked = files.len();
    if files.is_empty() { return Ok(report); }
    if matches!(meta.software, Software::Vanilla | Software::Paper) {
        report.errors.push(format!("{} does not load mods from mods/. Choose Fabric, Forge or NeoForge; Paper plugins belong in plugins/.", meta.software.label()));
        return Ok(report);
    }
    let mut mods = Vec::<JarMod>::new();
    for path in files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let Some(mut zip) = File::open(&path).ok().and_then(|file| ZipArchive::new(file).ok()) else {
            report.warnings.push(format!("{name}: unreadable JAR; cannot verify compatibility."));
            continue;
        };
        let fabric = descriptor(&mut zip, "fabric.mod.json");
        let forge = descriptor(&mut zip, "META-INF/mods.toml");
        let neoforge = descriptor(&mut zip, "META-INF/neoforge.mods.toml");
        let quilt = descriptor(&mut zip, "quilt.mod.json");
        let selected = match meta.software {
            Software::Fabric => {
                if fabric.is_none() && (forge.is_some() || neoforge.is_some() || quilt.is_some()) {
                    report.errors.push(format!("{name}: this is not a Fabric mod."));
                }
                fabric.as_deref().and_then(|s| fabric_descriptor(s, &name))
            }
            Software::Forge => {
                if forge.is_none() && (fabric.is_some() || neoforge.is_some()) {
                    report.errors.push(format!("{name}: this is not a Forge mod."));
                }
                forge.as_deref().and_then(|s| forge_descriptor(s, &name))
            }
            Software::Neoforge | Software::Arclight => {
                if fabric.is_some() && forge.is_none() && neoforge.is_none() {
                    report.errors.push(format!("{name}: Fabric mod installed on NeoForge/Arclight."));
                }
                // Older NeoForge mods may still ship mods.toml.
                neoforge.as_deref().or(forge.as_deref()).and_then(|s| forge_descriptor(s, &name))
            }
            Software::Vanilla | Software::Paper => None,
        };
        if let Some(mod_jar) = selected {
            mods.push(mod_jar);
        } else if fabric.is_none() && forge.is_none() && neoforge.is_none() && quilt.is_none() {
            report.warnings.push(format!("{name}: no recognized mod metadata; check this file."));
        } else if !report.errors.iter().any(|e| e.starts_with(&name)) {
            report.warnings.push(format!("{name}: mod metadata could not be parsed."));
        }
    }
    let installed = mods.iter().flat_map(|m| m.ids.iter().cloned()).collect::<HashMap<String, String>>();
    let neoforge = neoforge_version(dir, meta.software, meta.loader_version.as_deref());
    for module in &mods {
        for dep in &module.dependencies {
            if module.ids.iter().any(|(id, _)| id == &dep.id) { continue; }
            let actual = match dep.id.as_str() {
                "minecraft" => Some(meta.minecraft_version.as_str()),
                "fabricloader" if meta.software == Software::Fabric => meta.loader_version.as_deref(),
                "forge" if meta.software == Software::Forge => meta.loader_version.as_deref(),
                "neoforge" if matches!(meta.software, Software::Neoforge | Software::Arclight) => neoforge.as_deref(),
                "java" => None, // Validated separately before launch.
                other => installed.get(other).map(String::as_str),
            };
            if let Some(version) = actual {
                if matches_req(version, &dep.requirement) == Some(false) {
                    report.errors.push(format!("{} ({}) requires {} {}, installed {}.", module.filename, dep.owner, dep.id, dep.requirement, version));
                }
            } else if dep.id == "neoforge" && meta.software == Software::Arclight {
                report.warnings.push(format!("{} requires NeoForge {}, but Arclight's embedded version isn't yet detectable. Check libraries/net/neoforged/neoforge/.", module.filename, dep.requirement));
            } else if dep.id != "java" {
                report.warnings.push(format!("{} requires {} {}; not found as a separate JAR (may be bundled jar-in-jar).", module.filename, dep.id, dep.requirement));
            }
        }
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiversal_vs_neoforge_192() {
        let raw = r#"
            modLoader = "javafml"
            loaderVersion = "[4,)"
            license = "MIT"
            [[mods]]
            modId = "multiversal"
            version = "1.0.0"
            [[dependencies.multiversal]]
            modId = "neoforge"
            type = "required"
            versionRange = "[21.1.265,)"
            side = "BOTH"
        "#;
        let jar = forge_descriptor(raw, "multiversal.jar").unwrap();
        assert_eq!(jar.dependencies.len(), 1);
        assert_eq!(matches_req("21.1.192", &jar.dependencies[0].requirement), Some(false));
        assert_eq!(matches_req("21.1.265", &jar.dependencies[0].requirement), Some(true));
    }
    #[test]
    fn fabric_and_minecraft_ranges() {
        assert_eq!(matches_req("1.21.1", "[1.21.1,1.22)"), Some(true));
        assert_eq!(matches_req("1.22.0", "[1.21.1,1.22)"), Some(false));
        assert_eq!(matches_req("1.21.2", "1.21.1"), Some(false));
        assert_eq!(matches_req("0.16.5", ">=0.16.0"), Some(true));
        assert_eq!(matches_req("0.15.0", ">=0.16.0"), Some(false));
        assert!(fabric_descriptor(r#"{"id":"demo","version":"1.0.0","depends":{"minecraft":">=1.21.1"}}"#, "demo.jar").is_some());
    }
}