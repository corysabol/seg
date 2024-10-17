use clap::{Parser, Subcommand};
use std::io::Write;
use std::net::IpAddr;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::vec;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, UdpSocket};
use tokio::process::Command;
use tokio::signal::ctrl_c;

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
        /// Path to the file containing lines of network-name,listener-ip
        #[arg(short, long)]
        input_file: String,
        #[arg(short, long, value_enum, default_value = "both")]
        scan_type: Protocol,
    },
    /// Run in listener mode
    Listen {
        /// Use legacy non nf_tables based rules. NOT YET IMPLEMENTED!
        #[arg(long)]
        legacy: Option<bool>,
        /// The protocol to listen for connection over. NOT YET IMPLEMENTED!
        #[arg(long, value_enum, default_value = "both")]
        protocol: Protocol,
        #[arg(short, long, default_value = "0.0.0.0")]
        listen_address: String,
        /// Port used to access the host (typicall 22 for ssh)
        #[arg(short, long, default_value = "22")]
        access_port: String,
        /// Port to listen on for both TCP and UDP
        #[arg(short, long, default_value = "5555")]
        port: String,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum Protocol {
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
        Commands::Listen {
            legacy: _,
            protocol: _,
            listen_address,
            access_port,
            port,
        } => {
            // We need to hook up ctrl-c signal so that we can tear down the iptables rule when
            // exiting
            // TODO: better overall error handling so that we can clean up on failures
            run_listener(listen_address.clone(), access_port.clone(), port.clone()).await;
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

async fn run_scan(input_file: String, scan_type: Protocol) {
    let file = tokio::fs::File::open(&input_file)
        .await
        .expect("Unable to open input file.");
    let reader = tokio::io::BufReader::new(file);

    let scan_stats = Arc::new(Mutex::new(ScanStats::new()));

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

        {
            let mut stats = scan_stats.lock().unwrap();
            stats.networks_scanned += 1;
        }

        let scan_stats = Arc::clone(&scan_stats);
        let scan_type = scan_type.clone();

        scan_nmap(
            &listener_ip,
            &format!("scan_{}", network_name),
            scan_type,
            scan_stats,
        )
        .await;
    }
}

async fn run_command(command: &str, args: &[&str], input: Option<String>) -> tokio::io::Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        match input {
            Some(input) => {
                stdin
                    .write_all(input.as_bytes())
                    .await
                    .expect("Failed to write rules to nft stdin");
                stdin.shutdown().await.expect("Failed to flush stdin");
            }
            None => {}
        }
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "Command '{}' failed with status {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            command, output.status, stdout, stderr
        );
        return Err(tokio::io::Error::new(
            tokio::io::ErrorKind::Other,
            "Command execution failed",
        ));
    }
    Ok(())
}

async fn setup_nft_rules(access_port: &str, listener_port: &str) {
    let nft_rules = format!(
        r#"
flush ruleset

table ip nat {{
    chain prerouting {{
        type nat hook prerouting priority 0;
        # avoid locking ourselves out by not forwarding the access port
        tcp dport {access_port} counter accept
        tcp dport != {listener_port} counter redirect to :{listener_port}
        udp dport != {listener_port} counter redirect to :{listener_port}

    }}
}}

table ip filter {{
    chain input {{
        type filter hook input priority 0; policy accept;
        # Always accept listener port traffic
        tcp dport {access_port} counter accept
        tcp dport != {access_port} counter accept
        udp dport 1-65535 counter accept
    }}
}}"#
    );

    println!("Setting up firewall rules with nft:\n{}", nft_rules);

    // Create a temp file to use with nft
    let mut temp_file = tempfile::Builder::new()
        .prefix("seg_nft_rules")
        .suffix(".txt")
        .tempfile()
        .expect("Failed to open temp file for nft rules");

    write!(temp_file, "{}", nft_rules).expect("Failed to write nft rules to temp file");

    let temp_file_path = temp_file.path();
    let temp_file_path = temp_file_path.to_owned();
    let temp_file_path = temp_file_path
        .to_str()
        .expect("Failed to get temp file path");

    println!("Rules written to {:?}", temp_file_path);

    run_command("nft", &vec!["-f", temp_file_path], None)
        .await
        .expect("Failed to set rules with nft");
}

async fn teardown_nft_rules() {
    let cleanup_commands = [
        "delete table ip nat",
        "delete table ip filter",
        "flush ruleset",
    ];

    println!("Cleaning up nft rules...");
    for cmd in &cleanup_commands {
        if let Err(e) = run_command("nft", &[cmd], None).await {
            eprintln!("Failed to remove nft rules: {}, {}", cmd, e);
        }
    }
}

async fn scan_nmap(
    listener_ip: &str,
    output_file: &str,
    scan_type: Protocol,
    scan_stats: Arc<Mutex<ScanStats>>,
) {
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

async fn run_listener(address: String, access_port: String, port: String) {
    let access_port: u16 = access_port
        .parse()
        .expect(&format!("Error invalid port specification {}", port));
    let port: u16 = port
        .parse()
        .expect(&format!("Error invalid port specification {}", port));

    let address: IpAddr = address
        .parse()
        .expect(&format!("Invalid listener address specified {}", address));

    // Establish IP tables rules
    setup_nft_rules(
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
            teardown_nft_rules().await;
        },
    }
}

async fn write_to_log(
    log_writer: Arc<tokio::sync::Mutex<BufWriter<tokio::fs::File>>>,
    entry: String,
) {
    let mut writer = log_writer.lock().await;
    if let Err(e) = writer.write_all(entry.as_bytes()).await {
        eprintln!("Failed to write to log: {}", e);
    }
    if let Err(e) = writer.flush().await {
        eprintln!("Failed to flush log writer: {}", e);
    }
}

async fn handle_tcp(
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

async fn handle_udp(
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
