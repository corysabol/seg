// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    fs::File,
    io::{self, BufRead},
};

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
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, load_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
