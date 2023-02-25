use std::{str::{FromStr, Chars}, error::Error, fmt::{Debug, Display}, path::PathBuf};
use anyhow::{Result, anyhow};
use tokio::{join, fs};
use crate::misc::{
    list_all_nvm_versions,
    NVM_VERSION_DIR_OLD,
    NVM_VERSION_DIR_NEW,
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

impl Version {
    pub fn is_full(&self) -> bool {
        self.minor.is_some() && self.patch.is_some()
    }

    pub fn len(&self) -> usize {
        let has_minor = if self.minor.is_some() { 1 } else { 0 };
        let has_patch = if self.patch.is_some() { 1 } else { 0 };
        1 + has_minor + has_patch
    }

    pub fn matches(&self, other: &Self) -> bool {
        let (larger_version, smaller_version) = if other.len() > self.len() {
            (other, self)
        } else {
            (self, other)
        };

        if larger_version.major != smaller_version.major {
            false
        } else if larger_version.minor.is_none() || smaller_version.minor.is_none() {
            true
        } else if larger_version.minor.as_ref().unwrap() == smaller_version.minor.as_ref().unwrap() {
            if larger_version.patch.is_none() || smaller_version.patch.is_none() {
                true
            } else if larger_version.patch.as_ref().unwrap() == smaller_version.patch.as_ref().unwrap() {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl ToString for Version {
    fn to_string(&self) -> String {
        let mut output = self.major.to_string();

        if let Some(minor) = self.minor {
            output.push_str(".");
            output.push_str(minor.to_string().as_str());

            if let Some(patch) = self.patch {
                output.push_str(".");
                output.push_str(patch.to_string().as_str());
            }
        }

        format!("v{}", output)
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
                .map(|num| Some(num))
                .map_err(|_| ParseVersionError))
            .unwrap_or(Ok(None))?;
        let patch: Option<u32> = get_number_digits(&mut chr_iter)
            .map(|string| string.parse()
                .map(|num| Some(num))
                .map_err(|_| ParseVersionError))
            .unwrap_or(Ok(None))?;

        Ok(Version {
            major,
            minor,
            patch,
            location: None
        })
    }
}

/// Checks if a version exists
pub async fn find_version(version: &Version) -> Result<Version> {
    if version.is_full() {
        let version_string = version.to_string();
        let mut owned_version = version.clone();
        let old_version_dir_path = format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_OLD, version_string);
        let new_version_dir_path = format!("{}/{}/{}", HOME.as_str(), NVM_VERSION_DIR_NEW, version_string);
        let (old_version_dir, new_version_dir) = join!(
            fs::metadata(&old_version_dir_path),
            fs::metadata(&new_version_dir_path),
        );

        if let Ok(dir) = old_version_dir {
            if dir.is_dir() {
                let _ = owned_version.location.insert(PathBuf::from(old_version_dir_path));
                return Ok(owned_version);
            }
        }

        if let Ok(dir) = new_version_dir {
            if dir.is_dir() {
                let _ = owned_version.location.insert(PathBuf::from(new_version_dir_path));
                return Ok(owned_version);
            }
        }

        Err(anyhow!("couldn't find version {}", version.to_string()))
    } else {
        let mut versions = list_all_nvm_versions().await?;
        versions.retain(|version_entry| version.matches(version_entry));
        versions.sort_by(|a, b| a.partial_cmp(b).unwrap());
        versions.pop().ok_or(anyhow!("couldn't find version {}", version.to_string()))
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
