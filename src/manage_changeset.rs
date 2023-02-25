use crate::version::{Version, find_version};
use crate::env_utils::{EnvChangeset, get_new_env, strip_nvm_path};
use crate::misc::PATH;

pub async fn set_node_version(version: &Version) -> Vec<EnvChangeset> {
    let version = find_version(version).await.unwrap();
    let location = version.location.as_ref().unwrap().to_str().unwrap();
    let changesets = vec![
        EnvChangeset::UpdateVar {
            name: "PATH".to_string(),
            value: get_new_env(&version, strip_nvm_path(PATH.as_str()).as_str(), "/bin"),
        },
        EnvChangeset::UpdateVar {
            name: "NVM_BIN".to_string(),
            value: format!("{}/bin", location),
        },
        EnvChangeset::UpdateVar {
            name: "NVM_INC".to_string(),
            value: format!("{}/include/node", location),
        },
    ];

    changesets
}

/// I'll need this when I implement reverting to the system
/// install, which can be triggered from an .nvmrc file
pub fn revert_to_system_version() -> Vec<EnvChangeset> {
    let changesets = vec![
        EnvChangeset::UpdateVar {
            name: "PATH".to_string(),
            value: strip_nvm_path(PATH.as_str()),
        },
        EnvChangeset::DeleteVar { name: "NVM_BIN".to_string() },
        EnvChangeset::DeleteVar { name: "NVM_INC".to_string() },
    ];

    changesets
}
