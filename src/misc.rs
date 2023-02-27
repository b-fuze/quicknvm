use std::env::var as get_env_var;
use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use tokio::{join, fs};
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use futures_util::future::join_all;
use crate::version::Version;

pub const NVM_VERSION_DIR_OLD: &str = ".nvm";
pub const NVM_VERSION_DIR_NEW: &str = ".nvm/versions/node";
pub const NVM_VERSION_DIR_OLD_IOJS: &str = ".nvm/io.js";
pub const NVM_VERSION_DIR_NEW_IOJS: &str = ".nvm/versions/io.js";

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

const IOJS_MIN: Version = Version { major: 1, minor: None, patch: None, location: None };
const IOJS_MAX: Version = Version { major: 4, minor: None, patch: None, location: None };

pub fn is_iojs(version: &Version) -> bool {
    version >= &IOJS_MIN && version < &IOJS_MAX
}

pub fn get_runtime_name(version: &Version) -> &str {
    if is_iojs(version) { "io.js" } else { "node" }
}

/// This function is gnarly... It could use some serious refactoring
pub async fn list_all_nvm_versions() -> Result<Vec<Version>> {
    let (
        nvm_dir_old,
        nvm_dir_new,
        nvm_dir_old_iojs,
        nvm_dir_new_iojs,
    ) = join!(
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD)),
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW)),
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD_IOJS)),
        fs::read_dir(format!("{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW_IOJS)),
    );

    let streams = [
        nvm_dir_old,
        nvm_dir_new,
        nvm_dir_old_iojs,
        nvm_dir_new_iojs,
    ]
        .into_iter()
        .filter_map(|listing| listing
            .map(|read_dir| ReadDirStream::new(read_dir))
            .ok())
        .collect::<Vec<_>>();

    if streams.len() == 0 {
        return Err(anyhow!("no NVM directories could be read. Possibly invalid NVM install or permission issue"))
    }

    let version_entries = streams
        .into_iter()
        .map(|read_dir_stream| {
            read_dir_stream
                .filter(|entry| entry.is_ok())
                .map(|entry| entry.unwrap())
                .filter(|entry| entry
                    .file_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("v"))
                .collect::<Vec<_>>()
        });
    let mut version_entries = join_all(version_entries).await;
    let first_entries = version_entries.remove(0);
    let entries = version_entries
        .into_iter()
        .fold(first_entries, |mut acc, mut dir_entries| {
            acc.append(&mut dir_entries);
            acc
        });

    let mut versions = vec![];
    for entry in entries {
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

