//! Turns the OS socket table into a list of dev servers a phone might want to open.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::qr;

/// Ports that are always OS plumbing, even if a dev runtime somehow owns them.
const NOISE_PORTS: &[u16] = &[135, 139, 445, 5040, 5353, 5354, 5357, 7680];
/// OS services that sit on well-known dev ports, such as macOS AirPlay Receiver on 5000 and 7000.
const OS_PROCESSES: &[&str] = &["controlcenter", "airplayxpchelper", "rapportd", "sharingd"];
/// Processes that run web and app servers during development. Anything these own is shown.
const DEV_RUNTIMES: &[&str] = &[
    "node", "bun", "deno", "python", "python3", "pythonw", "py", "uvicorn", "gunicorn", "php",
    "php-cgi", "ruby", "java", "javaw", "dotnet", "iisexpress", "go", "air", "hugo", "caddy",
    "nginx", "httpd", "flutter", "dart", "dartaotruntime", "expo", "wslrelay", "com.docker.backend",
    "docker-proxy", "vpnkit", "trunk", "zola", "jekyll", "live-server", "http-server",
];
/// Ports dev tools default to, so a custom-named binary on one of them still shows.
const DEV_PORTS: &[u16] = &[
    1234, 1313, 3000, 3001, 3002, 3003, 4000, 4173, 4200, 4321, 4400, 5000, 5001, 5173, 5174,
    5175, 5500, 6006, 7000, 7070, 8000, 8001, 8008, 8010, 8080, 8081, 8088, 8100, 8443, 8787,
    8888, 9000, 9090, 19000, 19006,
];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServerEntry {
    pub port: u16,
    pub bind_addr: String,
    pub process_name: String,
    pub pid: u32,
    pub reachable_from_lan: bool,
    /// Plumbing a developer rarely cares about; shown only with "Show all".
    pub hidden: bool,
    pub url: String,
    /// Empty unless the server is reachable from the LAN (a localhost QR is useless on a phone).
    pub qr_svg: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Snapshot {
    pub lan_ip: Option<String>,
    pub servers: Vec<ServerEntry>,
    pub scanned_at: u64,
    pub error: Option<String>,
}

/// One listening socket, independent of the `listeners` crate so the logic below is testable.
#[derive(Debug, Clone)]
pub struct RawSocket {
    pub ip: IpAddr,
    pub port: u16,
    pub pid: u32,
    pub name: String,
}

/// Reads the socket table. Returns an error string rather than failing the whole app.
pub fn read_sockets() -> Result<Vec<RawSocket>, String> {
    let all = listeners::get_all().map_err(|e| e.to_string())?;
    Ok(all
        .into_iter()
        .filter(|l| l.protocol == listeners::Protocol::TCP && l.state == listeners::SocketState::Listen)
        .map(|l| RawSocket {
            ip: l.socket.ip(),
            port: l.socket.port(),
            pid: l.process.pid,
            name: l.process.name,
        })
        .collect())
}

/// Cheap identity of a scan, used to emit only when something actually changed.
pub fn fingerprint(sockets: &[RawSocket], lan_ip: Option<Ipv4Addr>) -> u64 {
    let set: BTreeSet<(u16, String, u32)> =
        sockets.iter().map(|s| (s.port, s.ip.to_string(), s.pid)).collect();
    let mut h = DefaultHasher::new();
    set.hash(&mut h);
    lan_ip.hash(&mut h);
    h.finish()
}

fn is_reachable(ip: &IpAddr, lan_ip: Option<Ipv4Addr>) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_unspecified() || Some(*v4) == lan_ip,
        // `::` on a dual-stack socket (Node's default for "all interfaces") accepts IPv4 too.
        IpAddr::V6(v6) => v6.is_unspecified() || v6.to_ipv4_mapped().is_some_and(|m| Some(m) == lan_ip),
    }
}

fn base_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

/// Shown by default only if a dev runtime owns it or it sits on a well-known dev port.
fn is_noise(port: u16, name: &str) -> bool {
    let base = base_name(name);
    let dev_runtime = DEV_RUNTIMES.contains(&base.as_str()) || base.starts_with("python");
    NOISE_PORTS.contains(&port)
        || port < 1024
        || OS_PROCESSES.contains(&base.as_str())
        || !(dev_runtime || DEV_PORTS.contains(&port))
}

fn display_name(name: &str) -> String {
    let trimmed = name.strip_suffix(".exe").unwrap_or(name);
    if trimmed.is_empty() { "unknown".to_string() } else { trimmed.to_string() }
}

pub fn build(sockets: &[RawSocket], lan_ip: Option<Ipv4Addr>) -> Vec<ServerEntry> {
    // One entry per port; IPv4 and IPv6 sockets for the same server collapse together.
    let mut by_port: BTreeMap<u16, Vec<&RawSocket>> = BTreeMap::new();
    for s in sockets {
        by_port.entry(s.port).or_default().push(s);
    }

    let mut entries: Vec<ServerEntry> = by_port
        .into_iter()
        .map(|(port, group)| {
            let best = group
                .iter()
                .copied()
                .max_by_key(|s| (is_reachable(&s.ip, lan_ip), s.ip.is_ipv4(), !s.name.is_empty()))
                .expect("group is never empty");
            let reachable = lan_ip.is_some() && is_reachable(&best.ip, lan_ip);
            let url = match (reachable, lan_ip) {
                (true, Some(ip)) => format!("http://{ip}:{port}"),
                _ => format!("http://localhost:{port}"),
            };
            ServerEntry {
                port,
                bind_addr: best.ip.to_string(),
                process_name: display_name(&best.name),
                pid: best.pid,
                reachable_from_lan: reachable,
                hidden: is_noise(port, &best.name),
                qr_svg: if reachable { qr::svg_for(&url) } else { String::new() },
                url,
            }
        })
        .collect();

    entries.sort_by_key(|e| (e.hidden, !e.reachable_from_lan, e.port));
    entries
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAN: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 5);

    fn sock(ip: &str, port: u16, name: &str) -> RawSocket {
        RawSocket { ip: ip.parse().unwrap(), port, pid: 100 + port as u32, name: name.into() }
    }

    #[test]
    fn wildcard_bind_is_reachable_with_lan_url_and_qr() {
        let out = build(&[sock("0.0.0.0", 3000, "node.exe")], Some(LAN));
        assert_eq!(out.len(), 1);
        assert!(out[0].reachable_from_lan);
        assert_eq!(out[0].url, "http://192.168.1.5:3000");
        assert_eq!(out[0].process_name, "node");
        assert!(out[0].qr_svg.contains("<svg"));
    }

    #[test]
    fn loopback_bind_is_localhost_only_without_qr() {
        let out = build(&[sock("::1", 5173, "node.exe")], Some(LAN));
        assert!(!out[0].reachable_from_lan);
        assert_eq!(out[0].url, "http://localhost:5173");
        assert!(out[0].qr_svg.is_empty());
    }

    #[test]
    fn dual_stack_sockets_collapse_and_prefer_reachable() {
        let out = build(&[sock("127.0.0.1", 8080, "python"), sock("::", 8080, "python")], Some(LAN));
        assert_eq!(out.len(), 1);
        assert!(out[0].reachable_from_lan);
        assert_eq!(out[0].bind_addr, "::");
    }

    #[test]
    fn bound_to_lan_ip_is_reachable() {
        let out = build(&[sock("192.168.1.5", 4200, "ng")], Some(LAN));
        assert!(out[0].reachable_from_lan);
    }

    #[test]
    fn no_network_means_nothing_is_reachable() {
        let out = build(&[sock("0.0.0.0", 3000, "node")], None);
        assert!(!out[0].reachable_from_lan);
        assert_eq!(out[0].url, "http://localhost:3000");
    }

    #[test]
    fn system_plumbing_is_hidden_and_sorted_last() {
        let out = build(
            &[sock("0.0.0.0", 135, "svchost.exe"), sock("0.0.0.0", 49664, "lsass.exe"), sock("::1", 5173, "node"), sock("127.0.0.1", 9180, "lghub_updater.exe"), sock("0.0.0.0", 2179, "vmms.exe"), sock("127.0.0.1", 41234, "node.exe")],
            Some(LAN),
        );
        let shown: Vec<u16> = out.iter().filter(|e| !e.hidden).map(|e| e.port).collect();
        assert_eq!(shown, vec![5173, 41234], "node on any port shows; background apps hide");
        assert!(out[2..].iter().all(|e| e.hidden), "hidden entries sort last");
    }

    #[test]
    fn macos_airplay_receiver_on_dev_ports_is_hidden() {
        let out = build(&[sock("0.0.0.0", 5000, "ControlCenter"), sock("::", 7000, "ControlCenter"), sock("0.0.0.0", 8080, "java")], Some(LAN));
        let shown: Vec<u16> = out.iter().filter(|e| !e.hidden).map(|e| e.port).collect();
        assert_eq!(shown, vec![8080]);
    }

    #[test]
    fn reachable_sorts_before_localhost_only() {
        let out = build(&[sock("::1", 3000, "node"), sock("0.0.0.0", 8000, "python")], Some(LAN));
        assert_eq!(out.iter().map(|e| e.port).collect::<Vec<_>>(), vec![8000, 3000]);
    }

    #[test]
    fn fingerprint_ignores_order_but_sees_changes() {
        let a = sock("0.0.0.0", 3000, "node");
        let b = sock("::1", 5173, "node");
        let f1 = fingerprint(&[a.clone(), b.clone()], Some(LAN));
        assert_eq!(f1, fingerprint(&[b.clone(), a.clone()], Some(LAN)));
        assert_ne!(f1, fingerprint(&[a.clone()], Some(LAN)));
        assert_ne!(f1, fingerprint(&[a, b], None));
    }
}

#[cfg(test)]
mod live {
    /// `cargo test live_scan -- --ignored --nocapture` prints what the app would show on this machine.
    #[test]
    #[ignore]
    fn live_scan() {
        let lan = crate::ip::lan_ipv4();
        let sockets = super::read_sockets().expect("socket table readable without admin");
        println!("LAN IP: {lan:?}  raw listening sockets: {}", sockets.len());
        for e in super::build(&sockets, lan) {
            println!("{:>5} {:<22} {:<16} lan={:<5} hidden={:<5} {}", e.port, e.process_name, e.bind_addr, e.reachable_from_lan, e.hidden, e.url);
        }
    }
}
