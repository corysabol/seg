use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Serialize, Deserialize, Debug)]
pub struct Connection {
    pub listener_ip: Ipv4Addr,
    pub network_tag: String,
    pub source_ip: Ipv4Addr,
    pub source_port: u16,
    pub target_port: u16,
    pub protocol: String, // "tcp" or "udp"
    pub timestamp: DateTime<Utc>,
}
