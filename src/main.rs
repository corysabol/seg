use clap::{Parser, Subcommand};
use std::fs::OpenOptions;
use std::net::IpAddr;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "Seg network segmentation scanner")]
#[command(author = "Cory Sabol")]
#[command(version = "0.1.0")]
#[command(about = "A tool to test network segmentation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in scanner mode
    Scan {
        /// Path to the file containing lines of network-name,listener-ip,network-range
        #[arg(short, long)]
        input_file: String,
        #[arg(short, long, value_enum)]
        scan_type: ScanType,
    },
    /// Run in listener mode
    Listen {
        #[arg(short, long, default_value = "0.0.0.0")]
        address: String,
        /// Port(s) to listen on for both TCP and UDP
        /// Can be of form n-m, n
        #[arg(short, long, default_value = "1-65535")]
        ports: String,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ScanType {
    TCP,
    UDP,
    BOTH,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan {
            input_file,
            scan_type,
        } => {
            run_scan(input_file.to_string(), scan_type.clone()).await;
        }
        Commands::Listen { address, ports } => {
            run_listener(address.clone(), ports.clone()).await;
        }
    }
}

struct ScanStats {
    networks_scanned: u32,
    hosts_scanned: u32,
    open_ports_found: u32,
}

impl ScanStats {
    fn new() -> Self {
        Self {
            networks_scanned: 0,
            hosts_scanned: 0,
            open_ports_found: 0,
        }
    }
}

async fn run_scan(input_file: String, scan_type: ScanType) {
    let file = tokio::fs::File::open(&input_file)
        .await
        .expect("Unable to open input file.");
    let reader = tokio::io::BufReader::new(file);

    let scan_stats = Arc::new(Mutex::new(ScanStats::new()));

    let mut lines = reader.lines();

    while let Some(Ok(entry)) = lines.next_line().await {
        // Parse the line and skip it if it isn't in the right format
        let parts: Vec<String> = entry.split(',').map(String::from).collect();
        if parts.len() != 3 {
            eprintln!("Invalid line format: {}", entry);
            continue;
        }

        let network_name = parts[0].clone();
        let listener_ip = parts[1].clone();

        println!("Starting scan for {}", entry);

        {
            let mut stats = scan_stats.lock().unwrap();
            stats.networks_scanned += 1;
        }

        let scan_stats = Arc::clone(&scan_stats);
        let scan_type = scan_type.clone();

        tokio::spawn(async move {
            scan_nmap(
                &listener_ip,
                &format!("scan_{}", network_name),
                scan_type,
                scan_stats,
            )
            .await;
        });
    }
}

async fn scan_nmap(
    listener_ip: &str,
    output_file: &str,
    scan_type: ScanType,
    scan_stats: Arc<Mutex<ScanStats>>,
) {
    let mut nmap_args = vec![];

    let output_file = match scan_type {
        ScanType::TCP => format!("{}_tcp", output_file),
        ScanType::UDP => format!("{}_udp", output_file),
        ScanType::BOTH => format!("{}_both", output_file),
    };

    match scan_type {
        ScanType::TCP => {
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
        ScanType::UDP => {
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
        ScanType::BOTH => {
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
        while let Some(Ok(line)) = lines.next() {
            if line.contains("open") {
                let mut stats = scan_stats.lock().unwrap();
                stats.open_ports_found += 1;
            }
            if line.contains("scanned") {
                let mut stats = scan_stats.lock().unwrap();
                stats.hosts_scanned += 1;
            }
            println!("{}", line); // Output line by line
        }
    }

    let exit_status = output.wait().expect("Nmap process failed.");
    if exit_status.success() {
        println!(
            "Scan completed for {}. Results saved to {}.nmap",
            listener_ip, output_file
        );
    } else {
        eprintln!("Nmap scan failed for {}", listener_ip);
    }
}

async fn run_listener(address: String, ports: String) {
    // Parse the ports
    let parts: Vec<&str> = ports.split('-').collect();
    if parts.len() > 2 || parts.len() < 1 {
        eprintln!("Error invalid port specification {}", ports);
        return;
    }

    let lower: u32 = parts[0]
        .parse()
        .expect(&format!("Error invalid port specification {}", ports));
    let upper: u32 = if parts.len() == 2 {
        parts[1]
            .parse()
            .expect(&format!("Error invalid port specification {}", ports))
    } else {
        lower
    };

    let address: IpAddr = address
        .parse()
        .expect(&format!("Invalid listener address specified {}", address));

    let log_file_path = "connections.log";
    let mut log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)
        .expect("Unable to open log file.");

    // Listen on specified port range
    for port in lower..=upper {
        let addr = address.clone();
        let log_file = log_file_path.to_string();

        tokio::spawn(async move {
            if let Ok(listener) = TcpListener::bind((addr, port)).await {
                loop {
                    match listener.accept().await {
                        Ok((socket, addr)) => {
                            let timestamp = SystemTime::now();
                            let log_entry =
                                format!("{:?}/tcp_connect/{:?}/{}", timestamp, addr, port);
                            println!("{}", log_entry);
                            let mut log_file = OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(log_file)
                                .expect("Unable to open log file.");
                            log_file
                                .write_all(log_entry.as_bytes())
                                .expect("Failed to write to log file.");
                        }
                        Err(e) => {
                            eprintln!("Failed to accept TCP connection on port {}: {}", port, e);
                        }
                    }
                }
            } else {
                eprintln!("Failed to bind TCP listener on port {}", port);
            }
        });

        let addr = address.clone();
        let log_file = log_file_path.clone();

        tokio::spawn(async move {
            if let Ok(udp_socket) = UdpSocket::bind((addr, port)).await {
                let mut buf = [0; 1024];
                loop {
                    match udp_socket.recv_from(&mut buf).await {
                        Ok((amt, src)) => {
                            let timestamp = SystemTime::now();
                            let log_entry =
                                format!("{:?}/udp_connect/{:?}/{}", timestamp, src, port);
                            println!("{}", log_entry);
                            let mut log_file = OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(log_file)
                                .expect("Unable to open log file.");
                            log_file
                                .write_all(log_entry.as_bytes())
                                .expect("Failed to write to log file.");
                        }
                        Err(e) => {
                            eprintln!("Failed to receive UDP packet on port {}: {}", port, e);
                        }
                    }
                }
            } else {
                eprintln!("Failed to bind UDP socket on port {}", port);
            }
        });
    }
}
