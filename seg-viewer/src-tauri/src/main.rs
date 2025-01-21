// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use data::PacketInfo;
use std::{
    collections::HashSet,
    env,
    fs::File,
    hash::{Hash, Hasher},
    io::{self, BufRead},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
struct NodeDatum {
    id: String,
    label: String,
    shape: String,
    color: String,
}

#[derive(Clone, Debug, Serialize)]
struct LinkDatum {
    id: String,
    label: String,
    source: String,
    target: String,
    active: bool,
    color: String,
}

#[derive(Clone, Debug, Serialize)]
struct GraphData {
    nodes: Vec<NodeDatum>,
    links: Vec<LinkDatum>,
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn load_data(file_path: &str) -> String {
    println!("Loading file {}", file_path);

    let file = File::open(file_path).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut nodes: HashSet<NodeDatum> = HashSet::new();
    let mut links: Vec<LinkDatum> = Vec::new();

    for (idx, line) in lines.enumerate() {
        let line = line.unwrap();
        let packet_info: PacketInfo = serde_json::from_str(line.as_str())
            .expect(format!("Failed to parse data - line number {:?}\n{:?}", idx, line,).as_str());

        // Gather the hosts as nodes

        // Source / Scanner Node
        nodes.insert(NodeDatum {
            id: format!("{}:{}", packet_info.source_ip.to_string(), "scanner"),
            label: format!(
                "{}:{}:{}",
                packet_info.network_tag,
                packet_info.source_ip.to_string(),
                "scanner",
            ),
            shape: "hexagon".to_string(),
            color: "#35D068".to_string(),
        });

        // Destination / listner node
        nodes.insert(NodeDatum {
            id: format!("{}:{}", packet_info.listener_ip.to_string(), "listener"),
            label: format!(
                "{}:{}:{}",
                packet_info.network_tag,
                packet_info.listener_ip.to_string(),
                "listener",
            ),
            shape: "square".to_string(),
            color: "#35D068".to_string(),
        });

        // Create links
        links.push(LinkDatum {
            id: format!(
                "{}:{}:{}",
                packet_info.source_ip.to_string(),
                packet_info.listener_ip.to_string(),
                packet_info.target_port.to_string(),
            ),
            label: format!("{} -> {}", packet_info.source_port, packet_info.target_port),
            source: format!("{}:{}", packet_info.source_ip.to_string(), "scanner"),
            target: format!("{}:{}", packet_info.listener_ip.to_string(), "listener"),
            active: true,
            color: "#35D068".to_string(),
        });
    }

    let nodes: Vec<_> = nodes.into_iter().collect();
    let graph = GraphData { nodes, links };

    serde_json::to_string(&graph)
        .expect(format!("Failed to serialize graph data: {:?}", graph).as_str())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, load_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
