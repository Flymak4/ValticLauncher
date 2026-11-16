//! Public Java Edition hosting via the bore.pub TCP relay (https://github.com/ekzhang/bore).
//!
//! Valtic implements the documented bore wire protocol directly, so the host only installs
//! Valtic Launcher. No router mapping, administrator access, extra executable, or VPN is used.
//! The third-party relay is best-effort, assigns an ephemeral port, and supports TCP/Java only.
//! This is not an implementation of VoxelPort or permission to use VoxelPort's relays.

use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, LazyLock, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const RELAY_HOST: &str = "bore.pub";
const RELAY_PORT: u16 = 7835;
const MAX_FRAME: usize = 256;
const MAX_SESSIONS: usize = 64;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    Checking,
    Public { address: String },
    Unavailable { reason: String },
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static CONNECTIONS: LazyLock<Mutex<HashMap<PathBuf, (u64, Connection)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn current(dir: &Path) -> Option<Connection> {
    CONNECTIONS.lock().ok()?.get(dir).map(|(_, value)| value.clone())
}

fn current_job(dir: &Path, id: u64) -> bool {
    CONNECTIONS.lock().is_ok_and(|all| all.get(dir).is_some_and(|(n, _)| *n == id))
}

fn set(dir: &Path, id: u64, state: Connection) -> bool {
    let Ok(mut all) = CONNECTIONS.lock() else {
        return false;
    };
    let Some((n, existing)) = all.get_mut(dir) else {
        return false;
    };
    if *n != id {
        return false;
    }
    *existing = state;
    true
}

/// Cancels a tunnel. The control socket is checked at least once per second.
pub fn stop(dir: &Path) {
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.remove(dir);
    }
}

fn finish(dir: &Path, id: u64) {
    if let Ok(mut all) = CONNECTIONS.lock() {
        if all.get(dir).is_some_and(|(n, _)| *n == id) {
            all.remove(dir);
        }
    }
}

/// Connect outwards only. The relay's control protocol listens on TCP port 7835.
fn dial() -> Result<TcpStream, String> {
    let addresses = (RELAY_HOST, RELAY_PORT)
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve the free relay: {e}"))?;
    let mut last = "The free relay is unavailable.".to_string();
    for address in addresses {
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(socket) => return Ok(socket),
            Err(e) => last = format!("Could not connect to the free relay: {e}"),
        }
    }
    Err(last)
}

/// The bore control frames are JSON with a single NUL byte delimiter.
fn send_frame(stream: &mut TcpStream, message: &Value) -> Result<(), String> {
    let data = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    if data.len() > MAX_FRAME {
        return Err("Relay message exceeds the protocol limit.".into());
    }
    stream.write_all(&data).and_then(|_| stream.write_all(&[0]))
        .map_err(|e| format!("Could not send to the relay: {e}"))
}

fn read_frame(stream: &mut TcpStream, pending: &mut Vec<u8>) -> Result<Option<Value>, String> {
    loop {
        if let Some(end) = pending.iter().position(|b| *b == 0) {
            if end > MAX_FRAME {
                return Err("Relay message exceeds the protocol limit.".into());
            }
            let data: Vec<u8> = pending.drain(..=end).collect();
            return serde_json::from_slice(&data[..end])
                .map(Some)
                .map_err(|_| "Unexpected relay message.".to_string());
        }
        if pending.len() > MAX_FRAME {
            return Err("Relay message exceeds the protocol limit.".into());
        }
        let mut bytes = [0_u8; 512];
        match stream.read(&mut bytes) {
            Ok(0) => return Err("The free relay closed the connection.".into()),
            Ok(n) => pending.extend_from_slice(&bytes[..n]),
            Err(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) => {
                return Ok(None);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(format!("Lost the public connection: {e}")),
        }
    }
}

fn relay_error(message: &Value) -> Option<String> {
    message.get("Error").and_then(Value::as_str).map(|s| s.chars().take(180).collect())
}

struct Slot(Arc<AtomicUsize>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A public player is paired with a fresh local loopback TCP connection.
/// Each direction is copied independently so Minecraft traffic is unmodified.
fn forward(id: String, port: u16) -> Result<(), String> {
    let local_addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut local = TcpStream::connect_timeout(&local_addr, CONNECT_TIMEOUT)
        .map_err(|e| e.to_string())?;
    let mut relay = dial()?;
    send_frame(&mut relay, &json!({ "Accept": id }))?;

    let mut relay_out = relay.try_clone().map_err(|e| e.to_string())?;
    let mut local_out = local.try_clone().map_err(|e| e.to_string())?;
    let incoming = thread::spawn(move || {
        let _ = io::copy(&mut relay, &mut local_out);
        let _ = local_out.shutdown(Shutdown::Write);
    });
    let _ = io::copy(&mut local, &mut relay_out);
    let _ = relay_out.shutdown(Shutdown::Write);
    let _ = incoming.join();
    Ok(())
}

fn run_relay(dir: &Path, id: u64, local_port: u16) -> Result<(), String> {
    let mut control = dial()?;
    control
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    send_frame(&mut control, &json!({ "Hello": 0 }))?;

    let mut pending = Vec::new();
    let started = Instant::now();
    let public_port = loop {
        if !current_job(dir, id) || !super::java::is_running(dir) {
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(10) {
            return Err("The free relay did not assign a public port.".into());
        }
        match read_frame(&mut control, &mut pending)? {
            Some(message) => {
                if let Some(reason) = relay_error(&message) {
                    return Err(reason);
                }
                if let Some(p) = message.get("Hello").and_then(Value::as_u64)
                    .and_then(|p| u16::try_from(p).ok()).filter(|p| *p != 0)
                {
                    break p;
                }
            }
            None => {}
        }
    };
    let public = Connection::Public { address: format!("{RELAY_HOST}:{public_port}") };
    if !set(dir, id, public) {
        return Ok(());
    }

    let sessions = Arc::new(AtomicUsize::new(0));
    while current_job(dir, id) && super::java::is_running(dir) {
        let Some(message) = read_frame(&mut control, &mut pending)? else {
            continue;
        };
        if let Some(reason) = relay_error(&message) {
            return Err(reason);
        }
        let Some(connection_id) = message.get("Connection").and_then(Value::as_str) else {
            continue; // Heartbeats and unknown future protocol messages.
        };
        // The relay sends UUIDs. Never forward an arbitrary unbounded string.
        if connection_id.len() != 36
            || !connection_id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        {
            continue;
        }
        if sessions.fetch_add(1, Ordering::Relaxed) >= MAX_SESSIONS {
            sessions.fetch_sub(1, Ordering::Relaxed);
            continue;
        }
        let session_id = connection_id.to_owned();
        let sessions = Arc::clone(&sessions);
        thread::spawn(move || {
            let _slot = Slot(sessions);
            let _ = forward(session_id, local_port);
        });
    }
    Ok(())
}

/// Called after the Java process starts. Reconnects on a temporary relay failure.
/// The public address is updated only after a confirmed relay handshake.
pub fn start(dir: PathBuf, port: u16) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut all) = CONNECTIONS.lock() {
        all.insert(dir.clone(), (id, Connection::Checking));
    } else {
        return;
    }
    thread::spawn(move || {
        while current_job(&dir, id) && super::java::is_running(&dir) {
            match run_relay(&dir, id, port) {
                Ok(()) => break,
                Err(reason) => {
                    if !set(&dir, id, Connection::Unavailable { reason }) {
                        break;
                    }
                }
            }
            // Cancel promptly if the server stops; keep retries gentle on the shared relay.
            for _ in 0..20 {
                if !current_job(&dir, id) || !super::java::is_running(&dir) {
                    break;
                }
                thread::sleep(Duration::from_millis(250));
            }
            if !set(&dir, id, Connection::Checking) {
                break;
            }
        }
        finish(&dir, id);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn control_frames_are_delimited_and_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let sender = thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            peer.write_all(b"{\"Hello\":30400}\0\"Heartbeat\"\0").unwrap();
        });
        let mut stream = TcpStream::connect(address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut pending = Vec::new();
        assert_eq!(read_frame(&mut stream, &mut pending).unwrap(), Some(json!({ "Hello": 30400 })));
        assert_eq!(read_frame(&mut stream, &mut pending).unwrap(), Some(json!("Heartbeat")));
        sender.join().unwrap();
    }

    #[test]
    fn outbound_messages_follow_bore_control_protocol() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let sender = thread::spawn(move || {
            let mut peer = TcpStream::connect(address).unwrap();
            send_frame(&mut peer, &json!({ "Hello": 0 })).unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        assert_eq!(read_frame(&mut stream, &mut Vec::new()).unwrap(), Some(json!({ "Hello": 0 })));
        sender.join().unwrap();
    }
}
