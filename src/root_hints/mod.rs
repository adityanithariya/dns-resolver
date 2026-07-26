use std::net::{Ipv4Addr, SocketAddr};

/// The 13 root server letters, IPv4 addresses only for simplicity.
/// (IPv6 root addresses exist too but a v4-only resolver is a fine starting point.)
pub const ROOT_HINTS: &[(&str, Ipv4Addr)] = &[
    ("a.root-servers.net", Ipv4Addr::new(198, 41, 0, 4)),
    ("b.root-servers.net", Ipv4Addr::new(199, 9, 14, 201)),
    ("c.root-servers.net", Ipv4Addr::new(192, 33, 4, 12)),
    ("d.root-servers.net", Ipv4Addr::new(199, 7, 91, 13)),
    ("e.root-servers.net", Ipv4Addr::new(192, 203, 230, 10)),
    ("f.root-servers.net", Ipv4Addr::new(192, 5, 5, 241)),
    ("g.root-servers.net", Ipv4Addr::new(192, 112, 36, 4)),
    ("h.root-servers.net", Ipv4Addr::new(198, 97, 190, 53)),
    ("i.root-servers.net", Ipv4Addr::new(192, 36, 148, 17)),
    ("j.root-servers.net", Ipv4Addr::new(192, 58, 128, 30)),
    ("k.root-servers.net", Ipv4Addr::new(193, 0, 14, 129)),
    ("l.root-servers.net", Ipv4Addr::new(199, 7, 83, 42)),
    ("m.root-servers.net", Ipv4Addr::new(202, 12, 27, 33)),
];

pub fn root_server_addrs() -> Vec<SocketAddr> {
    ROOT_HINTS
        .iter()
        .map(|(_, ip)| SocketAddr::new((*ip).into(), 53))
        .collect()
}
