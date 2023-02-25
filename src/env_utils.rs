use crate::{version::Version, misc::DOT_NVM_HOME};

#[derive(Debug)]
pub enum EnvChangeset {
    UpdateVar {
        name: String,
        value: String,
    },
    DeleteVar {
        name: String,
    },
}

pub fn get_new_env(version: &Version, env: &str, append_path: &str) -> String {
    format!("{}{}:{}", version.location.as_ref().unwrap().to_str().unwrap(), append_path, env)
}

/// Remove any found NVM paths from a PATH env var string
pub fn strip_nvm_path(env_var: &str) -> String {
    let nvm_path = &DOT_NVM_HOME[..DOT_NVM_HOME.len() - 1];
    let stripped_env_var = env_var
        .split(":")
        .filter(|path| !path.starts_with(nvm_path))
        .collect::<Vec<_>>()
        .join(":");
    stripped_env_var
}

/// Generates a small shell script that the calling
/// shell can just `eval` to update its environment
pub fn gen_shell_script(changesets: &Vec<EnvChangeset>) -> String {
    let separator = "\n";
    let script = changesets
        .into_iter()
        .map(|changset| match changset {
            EnvChangeset::UpdateVar {
                name,
                value,
            } => {
                format!("export {}={};", name, sanitize_shell_value(&value))
            },
            EnvChangeset::DeleteVar { name } => {
                format!("unset {};", name)
            },
        })
        .fold(String::new(), |mut acc, item| {
            acc.push_str(separator);
            acc.push_str(&item);
            acc
        });

    script
}

pub fn sanitize_shell_value(value: &str) -> String {
    let mut sanitized_inner_string = String::new();
    for chr in value.chars() {
        match chr {
            '\'' => sanitized_inner_string.push_str("'\'$'"),
            '\r' => sanitized_inner_string.push_str("\\r"),
            '\n' => sanitized_inner_string.push_str("\\n"),
            '\\' => sanitized_inner_string.push_str("\\\\"),
            '\0' => {}, // we ignore null characters
            '\x1b' => sanitized_inner_string.push_str("\\e"),
            _ => sanitized_inner_string.push(chr),
        }
    }

    format!("$'{}'", sanitized_inner_string)
}
