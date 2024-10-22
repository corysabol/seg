use clap::{Parser, Subcommand};

mod consts;
mod firewall;
mod network;
mod util;

use consts::NFT_RULES_TEMPLATE;
use network::*;

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
        scan_type: ScanProtocol,
    },
    /// Run in listener mode
    Listen {
        /// Emits the base rules template for customization.
        #[arg(long, action = clap::ArgAction::SetTrue)]
        emit_rules: bool,
        /// An optional rules file to use
        #[arg(long)]
        rules: Option<String>,
        /// The protocol to listen for connection over. NOT YET IMPLEMENTED!
        #[arg(long, value_enum, default_value = "both")]
        protocol: ScanProtocol,
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
            emit_rules,
            rules,
            protocol: _,
            listen_address,
            access_port,
            port,
        } => {
            if *emit_rules {
                // Emit the rules file and exit
                println!(
                    "{}",
                    NFT_RULES_TEMPLATE
                        .replace("{access_port}", access_port)
                        .replace("{listener_port}", port)
                );
                std::process::exit(0);
            }
            run_listener(
                rules.clone(),
                listen_address.clone(),
                access_port.clone(),
                port.clone(),
            )
            .await;
        }
    }
}
