//! Official Valtic SMP NeoForge 1.21.1 client profile.
//! Imported only when the user chooses Install on the dedicated server tab.
use crate::instances::{self, Icon, Instance};
use base64::Engine;
use std::fs;

pub const GROUP: &str = "Valtic SMP Official";
const PACK: &str = include_str!("../assets/valtic_smp/official.mrpack.b64");

pub fn is_smp(i: &Instance) -> bool {
    i.group.as_deref() == Some(GROUP)
}

pub fn install(progress: &dyn Fn(&str, f32)) -> Result<Instance, String> {
    if instances::list().iter().any(is_smp) {
        return Err("Valtic SMP is already installed.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(PACK.trim())
        .map_err(|e| format!("Invalid embedded SMP modpack: {e}"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let file = std::env::temp_dir().join(format!("valticsmp-{}-{nonce}.mrpack", std::process::id()));
    fs::write(&file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    let imported = crate::instances_mrpack::import(&file, progress);
    let _ = fs::remove_file(&file);
    let instance = imported?;
    instances::edit(&instance.id, |meta| {
        meta.group = Some(GROUP.into());
        meta.icon = Some(Icon::Colour("#4b8f84".into()));
    })
}
