mod misc;
mod env_utils;
mod manage_changeset;
mod nvmrc;
mod version;

use std::env::current_dir;
use tokio::fs;
use manage_changeset::set_node_version;
use env_utils::gen_shell_script;

#[tokio::main]
async fn main() {
    let cwd = current_dir().expect("couldn't get CWD");
    let nvmrc = nvmrc::find_nvmrc(cwd).await;
    let current_node_version = version::find_current_version(misc::PATH.as_str());

    let changesets_and_version = if let Some(nvmrc_path) = nvmrc {
        let nvmrc_contents = fs::read_to_string(&nvmrc_path).await.expect("can't read nvmrc file");
        let nvmrc_version = nvmrc::resolve_nvmrc_version(
            nvmrc_contents.as_str(),
            0
        ).await.expect("couldn't resolve nvmrc version");

        let has_same_node_version = current_node_version
            .map_or(false, |current| {
                nvmrc_version.matches(&current)
            });

        if has_same_node_version {
            None
        } else {
            let message = format!(
                "Found '{}' with version <{}>",
                nvmrc_path.to_str().unwrap(),
                nvmrc_contents.trim()
            );
            Some((set_node_version(&nvmrc_version).await, nvmrc_version, message))
        }
    } else {
        let default_version = nvmrc::resolve_nvmrc_version("default", 0).await;
        // If we can't resolve what the default version is
        // then just give up
        if default_version.is_err() {
            // TODO: add verbosity option that will explain why
            // this has happened
            return
        }
        let default_version = default_version.unwrap();

        let has_same_node_version = current_node_version
            .map_or(false, |current| {
                default_version.matches(&current)
            });

        if has_same_node_version {
            None
        } else {
            let message = "Reverting to nvm default version".to_string();
            Some((set_node_version(&default_version).await, default_version, message))
        }
    };

    if let Some((changesets, version, message)) = changesets_and_version {
        let shell_script = gen_shell_script(&changesets);
        println!("{}", &shell_script[1..]);

        // Human readable message
        eprintln!("{}", message);
        eprintln!("Now using node {}", version.to_string());
    }
}
