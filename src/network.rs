use crate::consts::*;
use crate::data::*;
use crate::firewall::*;
use crate::util::*;

use pnet::datalink::{self, NetworkInterface};

use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpFlags;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::process::Command;
use tokio::signal::ctrl_c;
use tokio::sync::Semaphore;
use tokio::time::timeout;

#[derive(Clone, clap::ValueEnum)]
pub enum ScanProtocol {
    TCP,
    UDP,
    BOTH,
}

enum ScannerSocketIpv4 {
    TCPSocket(TcpStream),
    UDPSocket(UdpSocket),
}

struct Scanner {
    target: String,
    semaphore: Arc<Semaphore>,
    timeout_duration: Duration,
}

impl Scanner {
    fn new(target: String, timeout_duration: Duration) -> Self {
        Self {
            target,
            semaphore: Arc::new(Semaphore::new(MAX_SOCKETS.into())),
            timeout_duration,
        }
    }

    async fn scan_ports(&self, lower_port: u16, upper_port: u16) {
        let mut tcp_handles = vec![];
        let mut udp_handles = vec![];

        for port in lower_port..=upper_port {
            // TCP
            let semaphore = self.semaphore.clone();
            let target = self.target.clone();
            let timeout_duration = self.timeout_duration.clone();
            let tcp_handle = tokio::spawn(async move {
                let scanner = Scanner {
                    target,
                    semaphore,
                    timeout_duration,
                };
                scanner.scan_tcp_port(port).await
            });
            tcp_handles.push(tcp_handle);

            // UDP
            let semaphore = self.semaphore.clone();
            let target = self.target.clone();
            let timeout_duration = self.timeout_duration.clone();
            let udp_handle = tokio::spawn(async move {
                let scanner = Scanner {
                    target,
                    semaphore,
                    timeout_duration,
                };
                scanner.scan_udp_port(port).await
            });
            udp_handles.push(udp_handle);
        }

        // Wait for tasks
        for handle in tcp_handles {
            let _ = handle.await;
        }
        for handle in udp_handles {
            let _ = handle.await;
        }
    }

    async fn scan_udp_port(&self, port: u16) {
        // This will wait until a permit can be grabbed
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("Failed to acquire permit");
        let addr = format!("{}:{}", self.target, port);
        let addr: SocketAddr = addr
            .parse()
            .expect(format!("Failed to parse address {:?}", addr).as_str());

        match timeout(self.timeout_duration, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                println!("tcp/{}/{}", addr, port);
            }
            _ => {}
        }
    }

    async fn scan_tcp_port(&self, port: u16) {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("Failed to acquire permit");

        // TODO: consider UDP socket reuse
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("Failed to bind UDP socket");

        let addr = format!("{}:{}", self.target, port);
        match socket.connect(addr.clone()).await {
            Ok(_) => {
                // Send probe
                match socket.send(&[0; 1]).await {
                    Ok(_) => println!("udp/{}/{}", addr, port),
                    Err(_) => {}
                }
            }
            Err(_) => {}
        };
    }
}

pub async fn run_scan(input_file: String, scan_type: ScanProtocol) {
    let file = tokio::fs::File::open(&input_file)
        .await
        .expect("Unable to open input file.");
    let reader = tokio::io::BufReader::new(file);

    let mut lines = reader.lines();

    while let Ok(Some(entry)) = lines.next_line().await {
        // Parse the line and skip it if it isn't in the right format
        let parts: Vec<String> = entry.split(',').map(String::from).collect();
        if parts.len() != 2 {
            eprintln!("Invalid line format: {}", entry);
            continue;
        }

        let network_name = parts[0].clone();
        let listener_ip = parts[1].clone();

        println!("Starting scan for {}", entry);

        let scan_type = scan_type.clone();

        scan_nmap(&listener_ip, &format!("scan_{}", network_name), scan_type).await;
    }
}

pub async fn scan_nmap(listener_ip: &str, output_file: &str, scan_type: ScanProtocol) {
    let mut nmap_args = vec![];

    let output_file = match scan_type {
        ScanProtocol::TCP => format!("{}_tcp", output_file),
        ScanProtocol::UDP => format!("{}_udp", output_file),
        ScanProtocol::BOTH => format!("{}_both", output_file),
    };

    match scan_type {
        ScanProtocol::TCP => {
            nmap_args.extend(vec![
                "-p",
                "1-65535",
                "-sT",
                listener_ip,
                "--stats-every",
                "10s",
                "-oN",
                &output_file,
            ]);
        }
        ScanProtocol::UDP => {
            nmap_args.extend(vec![
                "-p",
                "1-65535",
                "-sU",
                listener_ip,
                "--stats-every",
                "10s",
                "-oN",
                &output_file,
            ]);
        }
        ScanProtocol::BOTH => {
            nmap_args.extend(vec![
                "-p",
                "1-65535",
                "-sT",
                "-sU",
                listener_ip,
                "--stats-every",
                "10s",
                "-oN",
                &output_file,
            ]);
        }
    }

    let nmap_args_owned: Vec<String> = nmap_args.into_iter().map(|s| s.to_string()).collect();

    let mut output = Command::new("nmap")
        .args(&nmap_args_owned)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute Nmap command.");

    if let Some(stdout) = output.stdout.take() {
        let reader = BufReader::new(stdout);

        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("{}", line); // Output line by line
        }
    }

    let exit_status = output.wait().await.expect("Nmap process failed.");
    if exit_status.success() {
        println!(
            "Scan completed for {}. Results saved to {}.nmap",
            listener_ip, output_file
        );
    } else {
        eprintln!("Nmap scan failed for {}", listener_ip);
    }
}

pub async fn run_listener(
    access_port: String,
    interface_name: String,
    network_tag: String,
    protocol: ScanProtocol,
) {
    println!("{}", access_port);
    let port: u16 = access_port.parse().expect(&format!(
        "Error invalid access port specification {}",
        access_port
    ));

    // Open log file and wrap it in a shared buffered writer for performance
    let log_file_path = "connections.log";
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)
        .await
        .expect("Unable to open log file.");

    let log_writer = BufWriter::new(log_file);
    let log_writer = Arc::new(tokio::sync::Mutex::new(log_writer));

    // Setup rules to accept all ports on UDP and TCP
    setup_firewall_rules(None, &access_port).await;

    tokio::select! {
        _ = handle_packet_log(interface_name, network_tag, port, protocol, log_writer) => {}
        _ = ctrl_c() => {
            println!("Shutting down... Cleaning up iptable rules");
            teardown_firewall_rules().await;
        },
    }
}

pub async fn handle_packet_log(
    interface_name: String,
    network_tag: String,
    access_port: u16,
    protocol: ScanProtocol,
    log_writer: Arc<tokio::sync::Mutex<BufWriter<tokio::fs::File>>>,
) {
    use pnet::datalink::Channel::Ethernet;

    // Find interfaces
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface: &NetworkInterface| {
            iface.is_up()
                && !iface.is_loopback()
                && iface.ips.len() > 0
                && iface.name == interface_name
        })
        .expect("Could not find interface");

    let local_ip = match interface.ips.iter().find(|ip| ip.is_ipv4()) {
        Some(ip) => match ip.ip() {
            std::net::IpAddr::V4(ipv4) => ipv4,
            _ => panic!("Expected an IPv4 address"),
        },
        None => panic!("No IPv4 address found on interface"),
    };

    // Create a channel to receive on
    let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => todo!(),
        Err(_) => todo!(),
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet) = EthernetPacket::new(packet) {
                    // Handle IPv4 packets
                    if let Some(ip_packet) = Ipv4Packet::new(ethernet.payload()) {
                        if ip_packet.get_destination() == local_ip {
                            match ip_packet.get_next_level_protocol() {
                                IpNextHeaderProtocols::Udp => {
                                    if let Some(udp_packet) = UdpPacket::new(ip_packet.payload()) {
                                        let source_port = udp_packet.get_source();
                                        let destination_port = udp_packet.get_source();
                                        if source_port != access_port
                                            && destination_port != access_port
                                        {
                                            let connection_info = Connection {
                                                listener_ip: ip_packet.get_destination(),
                                                network_tag: network_tag.clone(),
                                                source_ip: ip_packet.get_source(),
                                                source_port: udp_packet.get_source(),
                                                target_port: udp_packet.get_destination(),
                                                protocol: "udp".to_string(),
                                                timestamp: chrono::Utc::now(),
                                            };
                                            write_connection_to_log(
                                                log_writer.clone(),
                                                &connection_info,
                                            )
                                            .await;
                                            println!(
                                                "UDP: {}:{} -> {}:{}",
                                                ip_packet.get_source(),
                                                udp_packet.get_source(),
                                                ip_packet.get_destination(),
                                                udp_packet.get_destination(),
                                            );
                                        }
                                    }
                                }
                                IpNextHeaderProtocols::Tcp => {
                                    if let Some(tcp_packet) = TcpPacket::new(ip_packet.payload()) {
                                        let source_port = tcp_packet.get_source();
                                        let destination_port = tcp_packet.get_source();
                                        let flags = tcp_packet.get_flags();
                                        if source_port != access_port
                                            && destination_port != access_port
                                        {
                                            if flags & (TcpFlags::RST | TcpFlags::ACK) == 0 {
                                                let source_port = tcp_packet.get_source();
                                                let destination_port = tcp_packet.get_source();
                                                if source_port != access_port
                                                    && destination_port != access_port
                                                {
                                                    let connection_info = Connection {
                                                        listener_ip: ip_packet.get_destination(),
                                                        network_tag: network_tag.clone(),
                                                        source_ip: ip_packet.get_source(),
                                                        source_port: tcp_packet.get_source(),
                                                        target_port: tcp_packet.get_destination(),
                                                        protocol: "tcp".to_string(),
                                                        timestamp: chrono::Utc::now(),
                                                    };
                                                    write_connection_to_log(
                                                        log_writer.clone(),
                                                        &connection_info,
                                                    )
                                                    .await;
                                                    println!(
                                                        "TCP: {}:{} -> {}:{}",
                                                        ip_packet.get_source(),
                                                        tcp_packet.get_source(),
                                                        ip_packet.get_destination(),
                                                        tcp_packet.get_destination(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
}
