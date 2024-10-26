//#[cfg(target_os = "linux")]
mod platform {
    use crate::consts::NFT_RULES_TEMPLATE;
    use crate::util::*;
    use std::io::Write;

    pub async fn setup_firewall_rules(rules: Option<String>, access_port: &str) {
        let nft_rules = match rules {
            Some(rules) => rules,
            None => NFT_RULES_TEMPLATE.replace("{access_port}", access_port),
        };

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

    pub async fn teardown_firewall_rules() {
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
}

// re-export platform
pub use platform::*;
