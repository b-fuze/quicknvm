# Quicknvm
[NVM](https://github.com/nvm-sh/nvm) auto-detection, but quick

This implements auto-detecting (**and** auto-installing) the correct
NVM version based on `.nvmrc` files, and does it in a way compatible
with NVM. It also properly detects IO.js installations in an
NVM-compatible way.

## Motivation
I use NVM, but it's kinda slow and wanted something fully
compatible but fast.

This only implements automatic Node version detection and
installation, it doesn't support commands of any sort.

## Install
[Get the Rust compiler](https://www.rust-lang.org/tools/install) if you
don't have it already and run
```sh
cargo build --release
```
Then copy the newly compiled binary at `target/release/quicknvm` to
somewhere in your `PATH`

## Usage
Add this to your `.zshrc`
```sh
autoload -U add-zsh-hook
load-quicknvm() {
  local new_version=$(quicknvm)

  if [[ $new_version ]]; then
    eval "$new_version"
  fi
}
add-zsh-hook chpwd load-quicknvm
load-quicknvm
```
## Supported `.nvmrc` values
Quicknvm should support most NVM `.nvmrc` supported values

| Syntax | Description | Example | Example outcome |
| --- | --- | --- | --- |
| `lts/codename` | LTS by codename | `lts/argon` | uses argon LTS |
| `lts/*` | latest LTS | `lts/*` | uses latest installed LTS (`hydrogen` at time of writing) |
| `lts/-N` | relative LTS | `lts/-3` | uses 3 LTS versions behind latest |
| `default` | default version | `default` | uses the default NVM version, [see below](#setting-the-default) |
| `system` | system-installed version | `system` | uses the system non-NVM managed version of Node.js if any |
| `stable` | latest stable version | `stable` | uses the latest stable installed version |
| `node` | latest stable version | `node` | uses the latest stable installed version |
| `iojs` | latest stable IO.js version | `iojs` | uses the latest stable installed IO.js version |

## Unsupported `.nvmrc` values
The only noteworthy value is probably `unstable` which is only
for Node.js pre-v1.

## License
MIT
