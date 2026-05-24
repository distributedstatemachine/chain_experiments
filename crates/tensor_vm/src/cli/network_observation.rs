use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(super) fn network_observation_multiaddr_is_public(address: &Multiaddr) -> bool {
    let mut saw_public_address = false;
    let mut saw_tcp_listen_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(ip) if public_ipv4(ip) => saw_public_address = true,
            Protocol::Ip6(ip) if public_ipv6(ip) => saw_public_address = true,
            Protocol::Dns(host) | Protocol::Dns4(host) | Protocol::Dns6(host)
                if public_dns_host(host.as_ref()) =>
            {
                saw_public_address = true;
            }
            Protocol::Tcp(port) if port != 0 => saw_tcp_listen_port = true,
            Protocol::Tcp(_) => {
                return false;
            }
            Protocol::Ip4(_)
            | Protocol::Ip6(_)
            | Protocol::Dns(_)
            | Protocol::Dns4(_)
            | Protocol::Dns6(_) => {
                return false;
            }
            _ => {}
        }
    }
    saw_public_address && saw_tcp_listen_port
}

fn public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();
    let is_shared_address_space = a == 100 && (64..=127).contains(&b);
    let is_protocol_assignment = a == 192 && b == 0 && c == 0;
    let is_documentation = (a == 192 && b == 0 && c == 2)
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113);
    let is_benchmarking = a == 198 && (b == 18 || b == 19);
    let is_multicast = (224..=239).contains(&a);
    let is_reserved_or_broadcast = (240..=255).contains(&a);
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || is_shared_address_space
        || is_protocol_assignment
        || is_documentation
        || is_benchmarking
        || is_multicast
        || is_reserved_or_broadcast)
}

fn public_ipv6(ip: Ipv6Addr) -> bool {
    let first_segment = ip.segments()[0];
    let unique_local = (first_segment & 0xfe00) == 0xfc00;
    let link_local = (first_segment & 0xffc0) == 0xfe80;
    let documentation = ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8;
    !(ip.is_unspecified()
        || ip.is_loopback()
        || unique_local
        || link_local
        || ip.is_multicast()
        || documentation)
}

pub(super) fn public_dns_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || special_use_dns_name(&host)
        || host.contains('@')
        || host
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => public_ipv4(ip),
        Ok(IpAddr::V6(ip)) => public_ipv6(ip),
        Err(_) => public_dns_host_is_well_formed(&host),
    }
}

fn special_use_dns_name(host: &str) -> bool {
    host == "local"
        || host == "test"
        || host == "example"
        || host == "invalid"
        || host == "example.com"
        || host == "example.net"
        || host == "example.org"
        || host.ends_with(".example.com")
        || host.ends_with(".example.net")
        || host.ends_with(".example.org")
        || host.ends_with(".test")
        || host.ends_with(".example")
        || host.ends_with(".invalid")
}

pub(super) fn public_dns_host_is_well_formed(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let mut label_count = 0;
    let mut labels = host.split('.').peekable();
    while let Some(label) = labels.next() {
        label_count += 1;
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let bytes = label.as_bytes();
        if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
            return false;
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return false;
        }
        if labels.peek().is_none() && !bytes.iter().any(|byte| byte.is_ascii_alphabetic()) {
            return false;
        }
    }
    label_count >= 2
}
