//! Free direct Minecraft hosting: try a router TCP mapping before offering Tailscale.
//! Only the player's own router is contacted. No Valtic relay or paid services.
use horizon_friends::{portmap, stun};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicU64, Ordering}, LazyLock, Mutex},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    Checking,
    Direct { address: String, method: String },
    Tailscale { reason: String },
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static CONNECTIONS: LazyLock<Mutex<HashMap<PathBuf, (u64, Connection)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn current(dir: &Path) -> Option<Connection> {
    CONNECTIONS.lock().ok()?.get(dir).map(|(_, c)| c.clone())
}

fn current_job(dir: &Path, id: u64) -> bool {
    CONNECTIONS.lock().is_ok_and(|s| s.get(dir).is_some_and(|(n, _)| *n == id))
}

fn set(dir: &Path, id: u64, state: Connection) -> bool {
    let Ok(mut all) = CONNECTIONS.lock() else { return false };
    let Some((n, existing)) = all.get_mut(dir) else { return false };
    if *n != id { return false; }
    *existing = state;
    true
}

/// Withdraw the public router mapping as soon as a locally started server stops.
/// The background worker owns the mapping, and notices this cancellation promptly.
pub fn stop(dir: &Path) {
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.remove(dir);
    }
}

fn internet_ipv4(ip: IpAddr) -> bool {
    let IpAddr::V4(ip) = ip else { return false };
    let [a,b,c,_] = ip.octets();
    !(
        a == 0 || a == 10 || a == 127 || a >= 224 ||
        (a == 100 && (64..=127).contains(&b)) ||
        (a == 169 && b == 254) ||
        (a == 172 && (16..=31).contains(&b)) ||
        (a == 192 && (b == 168 || (b == 0 && c == 0))) ||
        (a == 198 && (b == 18 || b == 19)) ||
        (a == 192 && b == 0 && c == 2) ||
        (a == 198 && b == 51 && c == 100) ||
        (a == 203 && b == 0 && c == 113)
    )
}

fn attempt(port: u16) -> Result<portmap::Mapping, String> {
    let ip = match super::lan_address() {
        Some(IpAddr::V4(v4)) if v4.is_private() => v4,
        _ => return Err("No local IPv4 network was detected for automatic router setup.".into()),
    };
    let targets = portmap::Targets::discover(Duration::from_secs(3));
    let mapping = portmap::map(&targets, ip, port, portmap::LEASE)?;
    if !internet_ipv4(mapping.external.ip()) {
        mapping.remove();
        return Err("The router did not provide a public IPv4 address (possibly carrier-grade NAT).".into());
    }
    // UPnP can succeed on an inner router even when the internet provider has another NAT.
    // STUN observes the actual outside address, so do not advertise a mismatched mapping.
    if let Some(seen) = stun::public_ip(&stun::servers_from_env(), Duration::from_secs(2)) {
        if seen != mapping.external.ip() {
            mapping.remove();
            return Err("The router's outside address differs from the internet address (double NAT or CGNAT).".into());
        }
    }
    Ok(mapping)
}

/// Called when a Java server starts. The mapping is renewed until the server stops.
pub fn start(dir: PathBuf, port: u16) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.insert(dir.clone(), (id, Connection::Checking));
    } else {
        return;
    }
    std::thread::spawn(move || {
        match attempt(port) {
            Err(reason) => {
                let _ = set(&dir, id, Connection::Tailscale { reason });
            }
            Ok(mut mapping) => {
                let address = if mapping.external.port() == 25565 {
                    mapping.external.ip().to_string()
                } else {
                    mapping.external.to_string()
                };
                if !set(&dir, id, Connection::Direct {
                    address, method: mapping.method.name().to_owned()
                }) {
                    mapping.remove();
                    return;
                }
                // Recheck state frequently so stopping a server removes the mapping.
                while current_job(&dir, id) {
                    std::thread::sleep(Duration::from_secs(5));
                    if !super::java::is_running(&dir) {
                        stop(&dir);
                        break;
                    }
                    if mapping.due() {
                        if let Err(e) = mapping.renew() {
                            let _ = set(&dir, id, Connection::Tailscale {
                                reason: format!("The router's port mapping expired: {e}")
                            });
                            break;
                        }
                    }
                }
                mapping.remove();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_advertise_private_or_cgnat_addresses() {
        for ip in ["192.168.1.1", "10.0.0.5", "100.64.0.1", "172.16.1.1",
                   "169.254.1.1", "127.0.0.1", "203.0.113.5"] {
            assert!(!internet_ipv4(ip.parse().unwrap()), "{ip}");
        }
        assert!(internet_ipv4("8.8.8.8".parse().unwrap()));
    }
}
