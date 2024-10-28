pub mod data {
    use chrono::{DateTime, Utc};
    use pnet::packet::tcp::TcpFlags;
    use serde::{Deserialize, Serialize};
    use std::net::Ipv4Addr;

    pub fn tcp_flags_to_iter(flags: u8) -> impl Iterator<Item = &'static str> {
        let mut flag_strings = vec![];

        if flags & TcpFlags::FIN != 0 {
            flag_strings.push("FIN");
        }
        if flags & TcpFlags::SYN != 0 {
            flag_strings.push("SYN");
        }
        if flags & TcpFlags::RST != 0 {
            flag_strings.push("RST");
        }
        if flags & TcpFlags::PSH != 0 {
            flag_strings.push("PSH");
        }
        if flags & TcpFlags::ACK != 0 {
            flag_strings.push("ACK");
        }
        if flags & TcpFlags::URG != 0 {
            flag_strings.push("URG");
        }
        if flags & TcpFlags::ECE != 0 {
            flag_strings.push("ECE");
        }
        if flags & TcpFlags::CWR != 0 {
            flag_strings.push("CWR");
        }

        flag_strings.into_iter()
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PacketInfo {
        pub listener_ip: Ipv4Addr,
        pub network_tag: String,
        pub source_ip: Ipv4Addr,
        pub source_port: u16,
        pub target_port: u16,
        pub protocol: String,   // "tcp" or "udp"
        pub flags: Vec<String>, // Todo need a good way to display the flags
        pub timestamp: DateTime<Utc>,
    }
}
