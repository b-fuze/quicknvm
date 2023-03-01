mod misc;
mod env_utils;
mod manage_changeset;
mod nvmrc;
mod version;
mod query_current;
mod install_node;

use std::env::current_dir;
use install_node::install_node;
use query_current::{npm_version, system_node_version};
use tokio::fs;
use manage_changeset::{set_node_version, revert_to_system_version};
use env_utils::gen_shell_script;
use version::{NodeVersion, find_version};
use misc::get_runtime_name;

#[tokio::main]
async fn main() {
    let cwd = current_dir().expect("couldn't get CWD");
    let nvmrc = nvmrc::find_nvmrc(cwd).await;
    let current_node_version = version::find_current_version(misc::PATH.as_str());

    // TODO: Refactor this into a `match tuple ...` somehow... Or just find some other
    // way to deduplicate it...
    let changesets_and_version = if let Some(nvmrc_path) = nvmrc {
        let nvmrc_contents = fs::read_to_string(&nvmrc_path).await.expect("couldn't read nvmrc file");
        let nvmrc_version = nvmrc::resolve_nvmrc_version(
            nvmrc_contents.as_str(),
            0
        ).await;

        let nvmrc_version = if let Ok(version) = nvmrc_version {
            version
        } else {
            eprintln!("Invalid .nvmrc '{}'", nvmrc_path.to_str().unwrap());
            return;
        };

        match nvmrc_version {
            NodeVersion::NvmVersion(version) => {
                let has_same_node_version = current_node_version
                    .map_or(false, |current| {
                        version
                            .as_ref()
                            .map(|version| version.matches(&current))
                            .unwrap_or(false)
                    });

                if has_same_node_version {
                    None
                } else {
                    let nvmrc_contents = nvmrc_contents.trim();
                    eprintln!(
                        "Found '{}' with version <{}>",
                        nvmrc_path.to_str().unwrap(),
                        nvmrc_contents
                    );
                    let version = match version {
                        Some(version) => version,
                        None => {
                            // We got an implicit version like `node`/`iojs`,
                            // install them first...
                            match install_node(nvmrc_contents).await {
                                Ok(version) => version,
                                _ => {
                                    eprintln!("Failed to install {}", nvmrc_contents.trim());
                                    return;
                                }
                            }
                        },
                    };

                    let installed_version = match find_version(&version).await {
                        Err(_) => {
                            // Node version isn't installed... Try installing it
                            // TODO: add some way to check if a version exists before installing it
                            let new_installed_version = install_node(nvmrc_contents).await;
                            if let Ok(version) = new_installed_version {
                                version
                            } else {
                                // Failed to install version
                                return;
                            }
                        },
                        Ok(version) => version
                    };

                    let npm_version = npm_version(Some(&installed_version))
                        .await
                        .map(|version| format!(" (npm {})", version))
                        .unwrap_or_else(|_| String::new());
                    eprintln!("Now using {} {}{}", get_runtime_name(&installed_version), installed_version, npm_version);
                    Some(set_node_version(&installed_version).await)
                }
            },

            NodeVersion::System => {
                if let Some(_) = current_node_version {
                    // We need to switch to the system version of Node
                    let system_version = system_node_version().await;
                    eprintln!(
                        "Found '{}' with version <{}>",
                        nvmrc_path.to_str().unwrap(),
                        nvmrc_contents.trim()
                    );
                    let version_message = if let Some(ref version) = system_version {
                        let npm_version = npm_version(None)
                            .await
                            .map(|version| format!(" (npm {})", version))
                            .unwrap_or_else(|_| String::new());
                        format!("Now using system version of Node: {}{}", version, npm_version)
                    } else {
                        "System version of node not found.".to_string()
                    };

                    eprintln!("{}", version_message);
                    Some(revert_to_system_version())
                } else {
                    // Already using the system version of Node
                    None
                }
            },
        }
    } else {
        let default_version = nvmrc::resolve_nvmrc_version("default", 0).await;
        // If we can't resolve what the default version is
        // then just give up
        if default_version.is_err() {
            // TODO: add verbosity option that will explain why
            // this has happened
            return;
        }
        let default_version = default_version.unwrap();

        match default_version {
            NodeVersion::NvmVersion(version) => {
                let has_same_node_version = current_node_version
                    .map_or(false, |current| {
                        version
                            .as_ref()
                            .map(|version| version.matches(&current))
                            .unwrap_or(false)
                    });

                if has_same_node_version {
                    None
                } else {
                    eprintln!("Reverting to nvm default version");
                    let version = match version {
                        Some(version) => version,
                        None => {
                            // We got an implicit version like `node`/`iojs`,
                            // install it first...
                            match install_node("default").await {
                                Ok(version) => version,
                                _ => {
                                    eprintln!("Failed to install default");
                                    return;
                                }
                            }
                        },
                    };
                    let installed_version = match find_version(&version).await {
                        Err(_) => {
                            // Node version isn't installed... Try installing it
                            // TODO: add some way to check if a version exists before installing it
                            let new_installed_version = install_node(version.to_string().as_str()).await;
                            if let Ok(version) = new_installed_version {
                                version
                            } else {
                                // Failed to install version
                                return;
                            }
                        },
                        Ok(version) => version
                    };

                    eprintln!("Now using {} {}", get_runtime_name(&installed_version), installed_version);
                    Some(set_node_version(&installed_version).await)
                }
            },
            NodeVersion::System => {
                if let Some(_) = current_node_version {
                    // We need to switch to the system version of Node
                    let system_version = system_node_version().await;
                    let version_message = if let Some(ref version) = system_version {
                        format!("Now using system version of Node: {}", version)
                    } else {
                        "Version 'system' not found - try `nvm ls-remote` to browse available versions.".to_string()
                    };

                    eprintln!("Reverting to nvm default version");
                    eprintln!("{}", version_message);

                    Some(revert_to_system_version())
                } else {
                    // Already using the system version of Node
                    None
                }
            },
        }
    };

    if let Some(changesets) = changesets_and_version {
        let shell_script = gen_shell_script(&changesets);
        println!("{}", &shell_script[1..]);
    }
}
