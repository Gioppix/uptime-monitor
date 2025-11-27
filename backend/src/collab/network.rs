use crate::eager_env;
use get_if_addrs::{IfAddr, get_if_addrs};
use std::net::{IpAddr, SocketAddr};

/// Checks if an IP address is acceptable (IPv6 ULA or IPv4 private).
fn is_acceptable_address(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V6(ipv6) => ipv6.is_unique_local(),
        IpAddr::V4(ipv4) => ipv4.is_private(),
    }
}

/// Gets the first private network address (prioritizing IPv6 ULA over IPv4 private).
///
/// Returns `None` if no private addresses are found or if retrieving interfaces fails.
pub fn get_first_network_address() -> Option<SocketAddr> {
    let mut if_addrs = get_if_addrs().ok()?;

    if_addrs.sort_by_key(|interface| match interface.addr {
        IfAddr::V6(_) => 0,
        IfAddr::V4(_) => 1,
    });

    for interface in if_addrs {
        let ip_addr = match interface.addr {
            IfAddr::V6(addr) => IpAddr::V6(addr.ip),
            IfAddr::V4(addr) => IpAddr::V4(addr.ip),
        };

        if is_acceptable_address(&ip_addr) {
            return Some(SocketAddr::new(ip_addr, *eager_env::PORT));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_get_first_network_address() {
        let result = get_first_network_address();
        println!("First network address: {:?}", result);
    }

    #[test]
    fn test_acceptable_addresses() {
        use std::net::Ipv6Addr;

        let fd12_ip: Ipv6Addr = "fd12:cd60:9071:1:1000:2d:8558:2c5f".parse().unwrap();
        let fe80_ip: Ipv6Addr = "fe80::a0aa:85ff:fe58:2c5f".parse().unwrap();

        assert!(
            is_acceptable_address(&IpAddr::V6(fd12_ip)),
            "fd12... (ULA) should be acceptable"
        );
        assert!(
            !is_acceptable_address(&IpAddr::V6(fe80_ip)),
            "fe80... (link-local) should not be acceptable"
        );
    }
}
