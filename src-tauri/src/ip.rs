//! Resolve the LAN IPv4 address a phone on the same network would use to reach this machine.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

/// Interface names that belong to virtual or VPN adapters a phone on the same Wi-Fi can never reach.
/// Covers Windows friendly names ("vEthernet (WSL)") and macOS/Linux device names ("virbr0").
const VIRTUAL_HINTS: &[&str] = &[
    "veth", "wsl", "hyper-v", "virtualbox", "vboxnet", "vmware", "vmnet", "docker", "br-",
    "virbr", "lxcbr", "lxdbr", "podman", "cni", "flannel", "loopback", "bluetooth", "awdl",
    "llw", "anpi", "tailscale", "zerotier", "wireguard", "wg0", "nordlynx", "nordvpn", "openvpn",
    "tap-", "proton", "mullvad", "expressvpn", "surfshark", "cloudflare", "warp", "vpn", "tunnel",
    "utun", "tun0",
];

pub fn lan_ipv4() -> Option<Ipv4Addr> {
    let routed = routed_ipv4().or_else(|| match local_ip_address::local_ip() {
        Ok(IpAddr::V4(v4)) => Some(v4),
        _ => None,
    });
    let interfaces: Vec<(String, Ipv4Addr)> = local_ip_address::list_afinet_netifas()
        .map(|list| {
            list.into_iter()
                .filter_map(|(name, ip)| match ip {
                    IpAddr::V4(v4) => Some((name, v4)),
                    IpAddr::V6(_) => None,
                })
                .collect()
        })
        .unwrap_or_default();
    choose(routed, &interfaces)
}

/// Asks the OS which source address it would route external traffic from.
/// `connect` on UDP only sets the default peer; no packet is sent.
fn routed_ipv4() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    match sock.local_addr().ok()?.ip() {
        IpAddr::V4(v4) => Some(v4),
        IpAddr::V6(_) => None,
    }
}

fn is_virtual(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    VIRTUAL_HINTS.iter().any(|hint| lower.contains(hint))
}

fn is_usable(ip: &Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
}

/// Lower is more likely to be the network a phone is on. Home routers hand out 192.168/16;
/// offices use 10/8; VPNs (NordLynx 10.5.0.2) and Hyper-V/WSL (172.16/12) crowd the rest.
fn rank(ip: &Ipv4Addr) -> u8 {
    match ip.octets() {
        [192, 168, ..] => 0,
        [10, ..] => 1,
        [172, b, ..] if (16..=31).contains(&b) => 2,
        _ => 3,
    }
}

/// Picks the best physical-looking address; the OS-routed address only breaks ties, because a
/// VPN usually owns the default route while the phone is on the physical LAN.
fn choose(routed: Option<Ipv4Addr>, interfaces: &[(String, Ipv4Addr)]) -> Option<Ipv4Addr> {
    let mut candidates: Vec<Ipv4Addr> = interfaces
        .iter()
        .filter(|(name, ip)| !is_virtual(name) && is_usable(ip))
        .map(|(_, ip)| *ip)
        .collect();
    if let Some(ip) = routed.filter(is_usable) {
        let known = interfaces.iter().any(|(_, i)| *i == ip);
        if !known {
            candidates.push(ip);
        }
    }
    candidates.into_iter().min_by_key(|ip| (rank(ip), Some(*ip) != routed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nic(name: &str, ip: &str) -> (String, Ipv4Addr) {
        (name.to_string(), ip.parse().unwrap())
    }

    #[test]
    fn routed_breaks_ties_between_two_home_nics() {
        let ifs = [nic("Wi-Fi", "192.168.1.5"), nic("Ethernet", "192.168.1.9")];
        assert_eq!(choose(Some("192.168.1.9".parse().unwrap()), &ifs), Some("192.168.1.9".parse().unwrap()));
    }

    #[test]
    fn ignores_vpn_that_owns_the_default_route() {
        let ifs = [nic("NordLynx", "10.5.0.2"), nic("vEthernet (WSL (Hyper-V firewall))", "172.22.48.1"), nic("Ethernet", "192.168.0.105")];
        assert_eq!(choose(Some("10.5.0.2".parse().unwrap()), &ifs), Some("192.168.0.105".parse().unwrap()));
    }

    #[test]
    fn skips_linux_and_macos_virtual_bridges() {
        let ifs = [nic("virbr0", "192.168.122.1"), nic("vmnet8", "192.168.56.1"), nic("docker0", "172.17.0.1"), nic("wlp2s0", "192.168.1.20")];
        assert_eq!(choose(Some("10.8.0.2".parse().unwrap()), &ifs), Some("192.168.1.20".parse().unwrap()));
        let mac = [nic("utun4", "10.5.0.2"), nic("en0", "192.168.4.31")];
        assert_eq!(choose(Some("10.5.0.2".parse().unwrap()), &mac), Some("192.168.4.31".parse().unwrap()));
    }

    #[test]
    fn office_10_network_is_used_when_no_home_range_exists() {
        let ifs = [nic("Ethernet", "10.20.30.40"), nic("vEthernet (Default Switch)", "172.22.240.1")];
        assert_eq!(choose(Some("10.20.30.40".parse().unwrap()), &ifs), Some("10.20.30.40".parse().unwrap()));
    }

    #[test]
    fn skips_hyperv_172_when_wifi_exists() {
        let ifs = [nic("vEthernet (Default Switch)", "172.20.0.1"), nic("Wi-Fi", "192.168.1.5")];
        assert_eq!(choose(Some("172.20.0.1".parse().unwrap()), &ifs), Some("192.168.1.5".parse().unwrap()));
    }

    #[test]
    fn keeps_172_office_network_when_it_is_all_there_is() {
        let ifs = [nic("Ethernet", "172.18.4.20")];
        assert_eq!(choose(Some("172.18.4.20".parse().unwrap()), &ifs), Some("172.18.4.20".parse().unwrap()));
    }

    #[test]
    fn rejects_link_local_and_loopback() {
        let ifs = [nic("Loopback Pseudo-Interface 1", "127.0.0.1"), nic("Ethernet", "169.254.3.3")];
        assert_eq!(choose(Some("169.254.3.3".parse().unwrap()), &ifs), None);
    }

    #[test]
    fn falls_back_to_interfaces_when_offline() {
        let ifs = [nic("Wi-Fi", "192.168.0.12")];
        assert_eq!(choose(None, &ifs), Some("192.168.0.12".parse().unwrap()));
    }
}
