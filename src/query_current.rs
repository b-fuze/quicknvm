use tokio::process::Command;
use crate::version::Version;
use crate::env_utils::strip_nvm_path;
use crate::misc::PATH;

pub async fn system_node_version() -> Option<Version> {
    let command = Command::new("node")
        // Remove any NVM dirs from the PATH before running the command to ensure
        // that the sytem Node.js (if any) is the actual one run
        .env("PATH", strip_nvm_path(PATH.as_str()))
        .arg("--version")
        .output().await;

    command
        .ok()
        .map(|output| {
            String::from_utf8(output.stdout)
                .map(|version_str| version_str.trim().parse().ok())
                .ok()
                .flatten()
        })
        .flatten()
}
