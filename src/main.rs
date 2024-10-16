use clap::{Parser, Subcommand};
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

async fn run_command(command: &str, args: &[&str]) -> tokio::io::Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

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

async fn setup_firewall(access_port: &str, listener_port: &str) {
    // Ensure the nat table and prerouting chain exist
    run_command("nft", &["add", "table", "ip", "nat"])
        .await
        .ok();
    run_command(
        "nft",
        &[
            "add",
            "chain",
            "ip",
            "nat",
            "prerouting",
            "{",
            "type",
            "nat",
            "hook",
            "prerouting",
            "priority",
            "0",
            ";",
            "}",
        ],
    )
    .await
    .ok();

    // Anti-lockout rule: allow access on the specified access port
    let anti_lockout_cmd = vec![
        "add",
        "rule",
        "ip",
        "nat",
        "prerouting",
        "tcp",
        "dport",
        access_port,
        "accept",
    ];

    // TCP redirection rule for other ports
    let tcp_redirection_cmd = vec![
        "add",
        "rule",
        "ip",
        "nat",
        "prerouting",
        "tcp",
        "dport",
        "1-65535",
        "dnat",
        "to",
        listener_port,
    ];

    // UDP redirection rule for other UDP ports
    let udp_redirection_cmd = vec![
        "add",
        "rule",
        "ip",
        "nat",
        "prerouting",
        "udp",
        "dport",
        "1-65535",
        "dnat",
        "to",
        listener_port,
    ];

    // Execute the rules
    run_command("nft", &anti_lockout_cmd)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to set anti-lockout rule: {}", e);
        });
    run_command("nft", &tcp_redirection_cmd)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to set TCP redirection rule: {}", e);
        });
    run_command("nft", &udp_redirection_cmd)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to set UDP redirection rule: {}", e);
        });
}

async fn teardown_firewall(access_port: &str, listener_port: &str) {
    // TODO: update to handle removing rules by their handles, rather than flushing all rules.
    let flush_cmd = vec!["flush", "chain", "ip", "nat", "prerouting"];

    if let Err(e) = run_command("nft", &flush_cmd).await {
        eprintln!("Failed to flush nftables chain: {}", e);
    }
}

async fn setup_firewall_legacy(access_port: &str, listener_port: &str) {
    // iptables -t nat -A PREROUTING -p tcp --dport 22 -j ACCEPT
    // iptables -t nat -A PREROUTING -p tcp -m multiport --dports 1:65535 -j DNAT --to-destination :2000
    let access_port = format!(":{}", access_port);
    let listener_port = format!(":{}", listener_port);

    let anti_lockout_args = vec![
        "-t",
        "nat",
        "-A",
        "PREROUTING",
        "-p",
        "tcp",
        "--dport",
        &access_port,
        "-j",
        "ACCEPT",
    ];

    let tcp_redirection_args = vec![
        "-t",
        "nat",
        "-A",
        "PREROUTING",
        "-p",
        "tcp",
        "-m",
        "multiport",
        "--dports",
        "1:65535",
        "-j",
        "NAT",
        "--to-destination",
        &listener_port,
    ];

    // Execute the iptables commands with output handling
    if let Err(e) = run_command("iptables", &anti_lockout_args).await {
        eprintln!("Failed to set iptables anti-lockout rule: {}", e);
    }

    if let Err(e) = run_command("iptables", &tcp_redirection_args).await {
        eprintln!("Failed to set iptables TCP redirection rule: {}", e);
    }

    // Modify the arguments for UDP redirection
    let mut udp_redirection_args = tcp_redirection_args.clone();
    udp_redirection_args[5] = "udp"; // Change protocol to UDP

    if let Err(e) = run_command("iptables", &udp_redirection_args).await {
        eprintln!("Failed to set iptables UDP redirection rule: {}", e);
    }
}

async fn teardown_firewall_legacy(access_port: &str, listener_port: &str) {
    // iptables -t nat -D PREROUTING -p tcp --dport 22 -j ACCEPT && iptables -t nat -D PREROUTING -p tcp -m multiport --dports 1:65535 -j DNAT --to-destination :5555
    let anti_lockout_args = vec![
        "-t",
        "nat",
        "-D",
        "PREROUTING",
        "-p",
        "tcp",
        "--dport",
        access_port,
        "-j",
        "ACCEPT",
    ];

    let tcp_redirection_args = vec![
        "-t",
        "nat",
        "-D",
        "PREROUTING",
        "-p",
        "tcp",
        "-m",
        "multiport",
        "--dports",
        "1:65535",
        "-j",
        "NAT",
        "--to-destination",
        &listener_port,
    ];

    // Avoid redirecting our access port such as 22 to the listener so we can still get into the
    // listener host.
    let mut args_clone = anti_lockout_args.clone();
    args_clone[7] = access_port;
    let output = Command::new("iptables")
        .args(&args_clone)
        .output()
        .await
        .expect("Failed to delete iptables rule");

    if !output.status.success() {
        eprintln!("Failed to remove iptables rule");
    }

    // Redirect all other TCP ports to the listener
    let output = Command::new("iptables")
        .args(&tcp_redirection_args)
        .output()
        .await
        .expect("Failed to delete TCP redirection rule");

    if !output.status.success() {
        eprintln!("Failed to remove iptables rule for TCP redirection");
    }

    // Redirect all UDP ports to the UDP listener
    let mut args_clone = tcp_redirection_args.clone();
    args_clone[5] = "udp";
    let output = Command::new("iptables")
        .args(&args_clone)
        .output()
        .await
        .expect("Failed to delete UDP redirection rule");

    if !output.status.success() {
        eprintln!("Failed to remove iptables rule for UDP redirection");
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
    setup_firewall(
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
            teardown_firewall(access_port.to_string().as_str(), port.to_string().as_str()).await;
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
                let timestamp = SystemTime::now();
                let log_entry = format!("{:?}/tcp_connect/{:?}", timestamp, addr);
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
                let timestamp = SystemTime::now();
                let log_entry = format!("{:?}/udp_connect/{:?}", timestamp, src);
                println!("{}", log_entry);
                write_to_log(log_writer.clone(), log_entry).await;
            }
            Err(e) => {
                eprintln!("Failed to receive UDP packet on port {}", e);
            }
        }
    }
}
