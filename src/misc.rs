use std::env::var as get_env_var;
use anyhow::Result;
use lazy_static::lazy_static;
use tokio::{join, fs};
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use super::version::Version;

pub const NVM_VERSION_DIR_OLD: &str = ".nvm";
pub const NVM_VERSION_DIR_NEW: &str = ".nvm/versions/node";

lazy_static! {
    pub static ref HOME: String = get_env_var("HOME")
        .expect("couldn't read HOME env var");
}

lazy_static! {
    pub static ref PATH: String = get_env_var("PATH")
        .expect("couldn't read PATH env var");
}

lazy_static! {
    pub static ref DOT_NVM_HOME: String = format!("{}/.nvm/", HOME.as_str());
}

pub async fn list_all_nvm_versions() -> Result<Vec<Version>> {
    let (nvm_dir_old, nvm_dir_new) = join!(
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD)),
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW)),
    );
    let old_stream = ReadDirStream::new(nvm_dir_old?);
    let new_stream = ReadDirStream::new(nvm_dir_new?);
    let version_entries = old_stream
        .chain(new_stream)
        .filter(|entry| entry.is_ok())
        .map(|entry| entry.unwrap())
        .filter(|entry| entry
            .file_name()
            .to_str()
            .unwrap_or("")
            .starts_with("v"))
        .collect::<Vec<_>>().await;
    let mut versions = vec![];

    for entry in version_entries {
        if entry.file_type().await?.is_dir() {
            let parsed_version = entry
                .file_name()
                .to_str()
                .unwrap()
                .parse::<Version>();
            if let Ok(mut version) = parsed_version {
                let _ = version.location.insert(entry.path());
                versions.push(version);
            }
        }
    }

    Ok(versions)
}

