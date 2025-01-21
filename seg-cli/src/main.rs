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
        /// The name / tag of the network
        #[arg(long, short)]
        network_tag: String,
        /// The interface to listen on
        #[arg(long, short)]
        interface_name: String,
        /// The protocol to listen for connection over.
        #[arg(long, value_enum, default_value = "both")]
        protocol: ScanProtocol,
        /// Port used to access the host (typically 22 for ssh)
        #[arg(short, long, default_value = "22")]
        access_port: String,
    },
    /// Parse seg JSONL scan data into various useful formats.
    Parse {
        /// The JSONL file of scan data to parse.
        #[arg(short, long)]
        input_file: String,
        /// A dir of JSONL files to parse.
        #[arg(short, long)]
        input_dir: String,
        /// Output as CSV.
        #[arg(long)]
        csv: bool,
        /// Output as Netflow.
        #[arg(long)]
        netflow: bool,
        /// Output file name (can be a path).
        #[arg(short, long)]
        out: String,
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
            network_tag,
            interface_name,
            protocol,
            access_port,
        } => {
            run_listener(
                access_port.clone(),
                interface_name.clone(),
                network_tag.clone(),
                protocol.clone(),
            )
            .await;
        }
    }
}
