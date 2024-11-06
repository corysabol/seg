use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Serialize, Deserialize, Debug)]
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
