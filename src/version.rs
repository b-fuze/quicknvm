use std::str::{FromStr, Chars};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use tokio::{join, fs};
use crate::misc::{
    list_all_nvm_versions,
    ListingType,
    NVM_VERSION_DIR_OLD,
    NVM_VERSION_DIR_NEW,
    NVM_VERSION_DIR_OLD_IOJS,
    NVM_VERSION_DIR_NEW_IOJS,
    DOT_NVM_HOME,
    HOME,
};

const INVALID_VERSION_STRING: &str = "invalid version";

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Version {
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub location: Option<PathBuf>,
}

pub enum NodeVersion {
    /// This is set to None when the provided nvmrc string
    /// is valid but a matching installation isn't found
    NvmVersion(Option<Version>),
    System,
}

impl Version {
    pub fn is_full(&self) -> bool {
        self.minor.is_some() && self.patch.is_some()
    }

    pub fn matches(&self, other: &Self) -> bool {
        if self.major != other.major {
            false
        } else if self.minor.is_none() || other.minor.is_none() {
            true
        } else if self.minor.as_ref().unwrap() == other.minor.as_ref().unwrap() {
            if self.patch.is_none() || other.patch.is_none() {
                true
            } else if self.patch.as_ref().unwrap() == other.patch.as_ref().unwrap() {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Version {
            major: 0,
            minor: None,
            patch: None,
            location: None
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = self.major.to_string();

        if let Some(minor) = self.minor {
            output.push_str(".");
            output.push_str(minor.to_string().as_str());

            if let Some(patch) = self.patch {
                output.push_str(".");
                output.push_str(patch.to_string().as_str());
            }
        }

        write!(f, "v{}", output)
    }
}

pub struct ParseVersionError;

impl Debug for ParseVersionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(INVALID_VERSION_STRING)
    }
}

impl Display for ParseVersionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(INVALID_VERSION_STRING)
    }
}

impl Error for ParseVersionError {}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(version_str: &str) -> Result<Self, Self::Err> {
        let mut version_str = version_str;

        if version_str.starts_with("v") {
            version_str = &version_str[1..];
        }

        if version_str.len() == 0 {
            return Err(ParseVersionError);
        }

        fn get_number_digits(iter: &mut Chars) -> Option<String> {
            let mut number = String::new();
            while let Some(digit) = iter.next() {
                match digit {
                    '.' => break,
                    _ => number.push(digit),
                }
            }

            if number.len() > 0 {
                Some(number)
            } else {
                None
            }
        }

        let mut chr_iter = version_str.chars();
        let parse_error = Err(ParseVersionError);

        let major: u32 = get_number_digits(&mut chr_iter)
            .map(|string| string.parse().map_err(|_| ParseVersionError))
            .unwrap_or(parse_error)?;
        let minor: Option<u32> = get_number_digits(&mut chr_iter)
            .map(|string| string.parse()
                .map_err(|_| ParseVersionError))
            .transpose()?;
        let patch: Option<u32> = get_number_digits(&mut chr_iter)
            .map(|string| string.parse()
                .map_err(|_| ParseVersionError))
            .transpose()?;

        Ok(Version {
            major,
            minor,
            patch,
            location: None
        })
    }
}

/// Checks if an NVM-managed Node version is installed
pub async fn find_version(version: &Version) -> Result<Version> {
    if version.is_full() {
        let version_string = version.to_string();
        let mut owned_version = version.clone();

        // TODO: See if there's a nice way to deduplicate all of this
        let paths = [
            format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD, version_string),
            format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW, version_string),
            format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD_IOJS, version_string),
            format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW_IOJS, version_string),
        ];
        // Search all possible locations concurrently
        let possible_version_paths = join!(
            fs::metadata(&paths[0]),
            fs::metadata(&paths[1]),
            fs::metadata(&paths[2]),
            fs::metadata(&paths[3]),
        );
        let possible_version_paths = [
            possible_version_paths.0,
            possible_version_paths.1,
            possible_version_paths.2,
            possible_version_paths.3,
        ];

        for (index, version) in possible_version_paths.iter().enumerate() {
            if let Ok(dir) = version {
                if dir.is_dir() {
                    let _ = owned_version.location.insert(PathBuf::from(&paths[index]));
                    return Ok(owned_version);
                }
            }
        }

        Err(anyhow!("couldn't find version {}", version))
    } else {
        let mut versions = list_all_nvm_versions(ListingType::Both).await?;
        versions.retain(|version_entry| version.matches(version_entry));
        versions.sort_by(|a, b| a.partial_cmp(b).unwrap());
        versions.pop().ok_or(anyhow!("couldn't find version {}", version))
    }
}

/// Checks to see what the current version is from
/// the PATH env variable. Returning None implies the
/// "system" version from NVM—i.e. there's no NVM-managed
/// Node.js configured/in the PATH
pub fn find_current_version(path: &str) -> Option<Version> {
    let nvm_path_dirs = path
        .split(":")
        .filter(|path| path.starts_with(DOT_NVM_HOME.as_str()))
        .map(|path| path.to_string())
        .next();

    if let Some(path) = nvm_path_dirs {
        let mut components: Vec<&str> = path
            .split("/")
            .filter(|component| component.len() > 0)
            .collect();
        components.pop();

        return components
            .pop()
            .map_or(None, |version| version.parse()
                .map_or(None, |mut version: Version| {
                    version.location = Some(PathBuf::from(&path[..path.len() - 4]));
                    Some(version)
                }));
    } else {
        None
    }
}
