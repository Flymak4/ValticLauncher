//! Valtic SMP's public Java status. This is independent of launcher authentication.
//! All network I/O happens off the UI thread; the Home card only reads the latest result.
use base64::Engine as _;
use serde_json::Value;
use std::time::Duration;

pub const ADDRESS: &str = "valticsmp.ultramc.co";
pub const REFRESH_SECS: u64 = 60;
const STATUS_URL: &str = "https://api.mcstatus.io/v2/status/java/valticsmp.ultramc.co?query=false&timeout=5";

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub online: bool,
    pub players: u64,
    pub capacity: u64,
    pub motd: String,
    pub icon: Option<Vec<u8>>,
}

/// Never assume a failed HTTP request means the Minecraft server is offline.
pub fn refresh() -> Option<Snapshot> {
    let response: Value = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build()
        .get(STATUS_URL)
        .call()
        .ok()?
        .into_json()
        .ok()?;
    let online = response.get("online")?.as_bool()?;
    let players = response.pointer("/players/online").and_then(Value::as_u64).unwrap_or(0);
    let capacity = response.pointer("/players/max").and_then(Value::as_u64).unwrap_or(0);
    let clean = response.pointer("/motd/clean").and_then(Value::as_str).unwrap_or("");
    let motd = clean
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join("  •  ")
        .chars()
        .take(110)
        .collect();
    let icon = response.get("icon")
        .and_then(Value::as_str)
        .and_then(|uri| uri.strip_prefix("data:image/png;base64,"))
        // Minecraft server icons are 64x64. Do not decode unbounded remote data.
        .filter(|encoded| encoded.len() <= 150_000)
        .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
        .filter(|bytes| bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    Some(Snapshot { online, players, capacity, motd, icon })
}

#[cfg(test)]
mod tests {
    #[test]
    fn server_address_is_the_valtic_java_address() {
        assert_eq!(super::ADDRESS, "valticsmp.ultramc.co");
    }
}
