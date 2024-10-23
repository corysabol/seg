use crate::consts::*;
use crate::data::*;
use crate::firewall::*;
use crate::util::*;

use std::net::IpAddr;
use std::net::SocketAddr;
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
use tokio::task;
use tokio::time::timeout;

#[derive(Clone, clap::ValueEnum)]
pub enum ScanProtocol {
    TCP,
    UDP,
    BOTH,
}

pub enum ScanType {
    TCP_SYN,
    TCP_FULL,
    TCP_ACK,
    ICMP,
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
                "-sS",
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
    rules: Option<String>,
    address: String,
    access_port: String,
    port: String,
    network_tag: String,
) {
    let access_port: u16 = access_port
        .parse()
        .expect(&format!("Error invalid port specification {}", port));
    let port: u16 = port
        .parse()
        .expect(&format!("Error invalid port specification {}", port));

    let address: IpAddr = address
        .parse()
        .expect(&format!("Invalid listener address specified {}", address));

    // Establish firewall rules for fowarding packets to the listener port
    setup_firewall_rules(
        rules,
        access_port.to_string().clone().as_str(),
        port.to_string().clone().as_str(),
    )
    .await;

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

    // Bind TCP and UDP listeners
    let addr = address.clone();

    let tcp_listener = TcpListener::bind((addr, port))
        .await
        .expect("Unable to bind TCP listener");
    let udp_listener = UdpSocket::bind((addr, port))
        .await
        .expect("Unable to bind UDP listener");

    // Create tokio tasks
    let tcp_task = handle_tcp(tcp_listener, log_writer.clone(), network_tag.clone());
    let udp_task = handle_udp(udp_listener, log_writer.clone(), network_tag.clone());

    tokio::select! {
        _ = tcp_task => {},
        _ = udp_task => {},
        _ = ctrl_c() => {
            println!("Shutting down... Cleaning up iptable rules");
            teardown_firewall_rules().await;
        },
    }
}

pub async fn handle_tcp(
    tcp_listener: TcpListener,
    log_writer: Arc<tokio::sync::Mutex<BufWriter<tokio::fs::File>>>,
    network_tag: String,
) {
    println!(
        "Listening for TCP connections on {:?}",
        tcp_listener
            .local_addr()
            .expect("Failed to get local addr of TCP listener")
    );

    loop {
        match tcp_listener.accept().await {
            Ok((_socket, addr)) => {
                let time = chrono::Utc::now();
                let connection = Connection {
                    listener_ip: tcp_listener
                        .local_addr()
                        .unwrap()
                        .ip()
                        .to_string()
                        .parse()
                        .unwrap(),
                    network_tag: network_tag.clone(),
                    source_ip: addr.ip().to_string().parse().unwrap(),
                    source_port: addr.port(),
                    target_port: tcp_listener.local_addr().unwrap().port(),
                    protocol: "tcp".to_string(),
                    timestamp: time,
                };
                println!("New connection {:?}", connection);
                write_connection_to_log(log_writer.clone(), &connection).await;
            }
            Err(e) => {
                eprintln!("Failed to accept TCP connection on port {}", e);
            }
        }
    }
}

pub async fn handle_udp(
    udp_socket: UdpSocket,
    log_writer: Arc<tokio::sync::Mutex<BufWriter<tokio::fs::File>>>,
    network_tag: String,
) {
    let mut buf = [0; 1024];
    println!(
        "Listening for UDP connections on {:?}",
        udp_socket
            .local_addr()
            .expect("Failed to get local addr of UDP listener")
    );
    loop {
        match udp_socket.recv_from(&mut buf).await {
            Ok((_amt, src)) => {
                let time = chrono::Utc::now();
                let connection = Connection {
                    listener_ip: udp_socket
                        .local_addr()
                        .unwrap()
                        .ip()
                        .to_string()
                        .parse()
                        .unwrap(),
                    network_tag: network_tag.clone(),
                    source_ip: src.ip().to_string().parse().unwrap(),
                    source_port: src.port(),
                    target_port: 0,
                    protocol: "udp".to_string(),
                    timestamp: time,
                };
                println!("New connection {:?}", connection);
                write_connection_to_log(log_writer.clone(), &connection).await;
            }
            Err(e) => {
                eprintln!("Failed to receive UDP packet on port {}", e);
            }
        }
    }
}
