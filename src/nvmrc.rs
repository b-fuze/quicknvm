use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use anyhow::{Context, Result, anyhow};
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use async_recursion::async_recursion;
use crate::misc::list_all_nvm_versions;
use crate::version::NodeVersion;

use crate::version::Version;
use crate::misc::HOME;

// TODO: enforce this for all file reads
const MAX_NVMRC_FILE_SIZE: u64 = 32;

/// Descends down from `starting_point` searching for an
/// `.nvmrc` file and stops at the first one that it finds
pub async fn find_nvmrc<T: AsRef<Path>>(starting_point: T) -> Option<PathBuf> {
    let mut path = starting_point.as_ref().to_path_buf();

    if !path.is_absolute() {
        return None;
    }

    // Remove the end if it's not a directory
    if let Ok(metadata) = fs::metadata(&path).await {
        if !metadata.is_dir() {
            path.pop();
        }
    } else {
        return None;
    }

    let mut stop_searching = false;
    loop {
        path.push(".nvmrc");

        let size = match fs::metadata(&path).await {
            Ok(metadata) => Some(metadata.size()),
            _ => None,
        };

        if let Some(size) = size {
            if size <= MAX_NVMRC_FILE_SIZE {
                return Some(path);
            }
            break;
        }

        path.pop();
        path.pop();

        if stop_searching { break }
        if path.parent().is_none() {
            // We've reached the root, so just search one more
            // time
            stop_searching = true;
        }
    }

    return None;
}

const LTS_STR_START: &str = "lts/";
const MAX_RECURSIVE_DEREF: u32 = 5;

#[async_recursion(?Send)]
pub async fn resolve_nvmrc_version(contents: &str, recursion_depth: u32) -> Result<NodeVersion> {
    if recursion_depth > MAX_RECURSIVE_DEREF {
        return Err(anyhow!("max nvmrc recursive lookup reached"));
    }

    let trimmed_contents = contents.trim();
    let home = HOME.as_str();

    if trimmed_contents.starts_with(LTS_STR_START) {
        let lts_str_len = LTS_STR_START.len();
        match trimmed_contents.get(lts_str_len..lts_str_len + 1) {
            // Relative LTS number (in the form of "lts/-N" with N being an integer)
            Some("-") => {
                let offset: usize = trimmed_contents[lts_str_len + 1..]
                    .parse()
                    .context("invalid relative nvmrc version")?;
                let nvm_lts_aliases = fs::read_dir(format!("{}/.nvm/alias/lts", home))
                    .await
                    .context("nvm LTS aliases not found (invalid nvm install?)")?;
                let mut nvm_lts_aliases = ReadDirStream::new(nvm_lts_aliases)
                    .filter_map(|dir| dir.map(|dir| Some(dir)).unwrap_or(None))
                    .filter(|dir| dir.file_name().to_str().unwrap() != "*")
                    .then(|dir| fs::read_to_string(format!("{}/.nvm/alias/lts/{}", home, dir.file_name().into_string().unwrap())))
                    .filter_map(|version_str| version_str
                        .map(|string| string.trim().parse::<Version>().map(|version| Some(version)).unwrap_or(None))
                        .unwrap_or(None))
                    .collect::<Vec<_>>()
                    .await;

                nvm_lts_aliases.sort_by(|a, b| a.partial_cmp(b).unwrap().reverse());
                return nvm_lts_aliases
                    .get(offset)
                    .map(|version| Ok(NodeVersion::NvmVersion(version.clone())))
                    .unwrap_or(Err(anyhow!("relative LTS version not found")));
            },
            // Normal LTS alias
            _ => {
                let lts_name = trimmed_contents[LTS_STR_START.len()..].trim();
                let path = format!("{}/.nvm/alias/lts/{}", home, lts_name);
                return resolve_nvmrc_version(fs::read_to_string(path).await?.as_str(), recursion_depth + 1).await;
            },
        }
    }

    match trimmed_contents {
        "node" | "stable" => {
            // Just sort the existing versions and find the latest
            let mut versions = list_all_nvm_versions().await?;
            versions.sort_by(|a, b| a.partial_cmp(b).unwrap());
            versions
                .pop()
                .map(|version| Ok(NodeVersion::NvmVersion(version)))
                .unwrap_or(Err(anyhow!("no versions found")))
        },
        "default" => {
            let path = format!("{}/.nvm/alias/default", home);
            return resolve_nvmrc_version(fs::read_to_string(path).await?.as_str(), recursion_depth + 1).await;
        },
        "system" => {
            return Ok(NodeVersion::System);
        },
        _ => {
            // Try to parse a Version struct
            trimmed_contents
                .parse()
                .map(|version| NodeVersion::NvmVersion(version))
                .context("invalid nvmrc version")
        }
    }
}
