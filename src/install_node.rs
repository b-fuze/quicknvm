use std::process::Stdio;
use anyhow::{anyhow, Result};
use tokio::process::Command;
use crate::version::{Version, find_version};
use crate::misc::DOT_NVM_HOME;

pub async fn install_node(version: &Version) -> Result<Version> {
    let nvm_script = format!("{}/nvm.sh", DOT_NVM_HOME.as_str());
    let install_script = r#"
        source "$1";
        nvm install "$2";
    "#;
    let install_command = Command::new("bash")
        .args(["-c", install_script, "--", nvm_script.as_str(), version.to_string().as_str()])
        .stdout(Stdio::null())
        .status()
        .await;

    if let Ok(status) = install_command {
        if status.success() {
            let version = find_version(&version).await?;
            Ok(version)
        } else {
            Err(anyhow!("version might not exist"))
        }
    } else {
        Err(anyhow!("failed to run `nvm install`"))
    }
}
