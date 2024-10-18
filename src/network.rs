use crate::firewall::*;
use crate::util::*;

use std::net::IpAddr;
use std::process::Stdio;
use std::sync::Arc;
use std::vec;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter};
use tokio::net::{TcpListener, UdpSocket};
use tokio::process::Command;
use tokio::signal::ctrl_c;

#[derive(Clone, clap::ValueEnum)]
pub enum Protocol {
    TCP,
    UDP,
    BOTH,
}

pub async fn run_scan(input_file: String, scan_type: Protocol) {
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

pub async fn scan_nmap(listener_ip: &str, output_file: &str, scan_type: Protocol) {
    let mut nmap_args = vec![];

    let output_file = match scan_type {
        Protocol::TCP => format!("{}_tcp", output_file),
        Protocol::UDP => format!("{}_udp", output_file),
        Protocol::BOTH => format!("{}_both", output_file),
    };

    match scan_type {
        Protocol::TCP => {
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
        Protocol::UDP => {
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
        Protocol::BOTH => {
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
    let tcp_task = handle_tcp(tcp_listener, log_writer.clone());
    let udp_task = handle_udp(udp_listener, log_writer.clone());

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
                let log_entry = format!("{}/tcp_connect/{:?}\n", time, addr);
                println!("{}", log_entry);
                write_to_log(log_writer.clone(), log_entry).await;
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
                let log_entry = format!("{:?}/udp_connect/{:?}\n", time, src);
                println!("{}", log_entry);
                write_to_log(log_writer.clone(), log_entry).await;
            }
            Err(e) => {
                eprintln!("Failed to receive UDP packet on port {}", e);
            }
        }
    }
}
