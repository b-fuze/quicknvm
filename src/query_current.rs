use anyhow::{anyhow, Result};
use futures_util::future::join_all;
use lazy_static::lazy_static;
use tokio::fs::{canonicalize, read_to_string};
use tokio::process::Command;
use serde::Deserialize;
use crate::version::Version;
use crate::env_utils::strip_nvm_path;
use crate::misc::PATH;

lazy_static! {
    static ref NO_NVM_PATH: String = strip_nvm_path(PATH.as_str());
}

#[derive(Deserialize)]
struct PackageJson {
    version: String,
}

/// Pass None to nvm_node_version to get the system NPM version
pub async fn npm_version(nvm_node_version: Option<&Version>) -> Result<Version> {
    let npm_executable_path = if let Some(version) = nvm_node_version {
        let node_version_dir = version.location.as_ref().unwrap().to_str().unwrap();
        let npm_nvm_path = format!("{}/bin/npm", node_version_dir);
        npm_nvm_path
    } else {
        let npm_system_path_futures = NO_NVM_PATH
            .as_str()
            .split(":")
            .map(|path| {
                let path = path.clone();
                async move {
                    // TODO: remove the double `canonicalize` call for system NPM
                    canonicalize(format!("{}/npm", path)).await
                }
            });
        let npm_system_path = join_all(npm_system_path_futures)
            .await
            .into_iter()
            .find_map(|path| path.ok());

        if let Some(path) = npm_system_path {
            path.to_str().unwrap().to_string()
        } else {
            return Err(anyhow!("no system NPM found"));
        }
    };

    let mut npm_package_json_path = canonicalize(&npm_executable_path).await?;
    npm_package_json_path.pop();
    npm_package_json_path.pop();
    npm_package_json_path.push("package.json");

    let package_json: PackageJson = serde_json::from_str(read_to_string(npm_package_json_path).await?.as_str())?;
    Ok(package_json.version.parse::<Version>()?)
}

pub async fn system_node_version() -> Option<Version> {
    let command = Command::new("node")
        // Remove any NVM dirs from the PATH before running the command to ensure
        // that the sytem Node.js (if any) is the actual one run
        .env("PATH", NO_NVM_PATH.as_str())
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
