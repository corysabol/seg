use chrono::Utc;
use serde_json::to_string;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::Command;

use crate::data;

pub async fn run_command(
    command: &str,
    args: &[&str],
    input: Option<String>,
) -> tokio::io::Result<()> {
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

pub async fn write_connection_to_log(
    log_writer: Arc<tokio::sync::Mutex<BufWriter<tokio::fs::File>>>,
    connection: &data::Connection,
) {
    let json = to_string(connection).unwrap();

    let mut writer = log_writer.lock().await;
    writer.write_all(json.as_bytes()).await.unwrap();
    writer.flush().await.unwrap();
}

pub async fn write_to_log(
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
