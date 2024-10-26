use clap::{Parser, Subcommand};

mod consts;
mod data;
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
        #[arg(long, short)]
        network_tag: String,
        /// The interface to listen on
        #[arg(long, short)]
        interface_name: String,
        /// The protocol to listen for connection over.
        #[arg(long, value_enum, default_value = "both")]
        protocol: ScanProtocol,
        /// Port used to access the host (typicall 22 for ssh)
        #[arg(short, long, default_value = "22")]
        access_port: String,
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
            network_tag,
            interface_name,
            protocol,
            access_port,
        } => {
            if *emit_rules {
                // Emit the rules file and exit
                println!(
                    "{}",
                    NFT_RULES_TEMPLATE
                        .replace("{access_port}", access_port)
                );
                std::process::exit(0);
            }
            run_listener(
                access_port.clone(),
                interface_name.clone(), 
                network_tag.clone(),
                protocol
            ).await;
        }
    }
}
