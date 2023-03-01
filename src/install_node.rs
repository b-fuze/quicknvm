use std::process::Stdio;
use anyhow::{anyhow, Result};
use tokio::process::Command;
use crate::nvmrc::resolve_nvmrc_version;
use crate::version::{Version, NodeVersion};
use crate::misc::DOT_NVM_HOME;

pub async fn install_node(version: &str) -> Result<Version> {
    let nvm_script = format!("{}/nvm.sh", DOT_NVM_HOME.as_str());
    let install_script = r#"
        source "$1";
        nvm install "$2";
    "#;
    let install_command = Command::new("bash")
        .args(["-c", install_script, "--", nvm_script.as_str(), version])
        .stdout(Stdio::null())
        .status()
        .await;

    if let Ok(status) = install_command {
        if status.success() {
            let downloaded_version = resolve_nvmrc_version(version, 0).await?;

            match downloaded_version {
                NodeVersion::NvmVersion(Some(version)) => Ok(version),
                _ => Err(anyhow!("unknown error installed '{}'", version))
            }
        } else {
            Err(anyhow!("version might not exist"))
        }
    } else {
        Err(anyhow!("failed to run `nvm install`"))
    }
}
