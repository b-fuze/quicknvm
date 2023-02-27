# Quicknvm
[NVM](https://github.com/nvm-sh/nvm) auto-detection, but quick

This implements auto-detecting (**and** auto-installing) the correct
NVM version based on `.nvmrc` files, and does it in a way compatible
with NVM. It also properly detects IO.js installations in an
NVM-compatible way.

## Motivation
I use NVM, but it's kinda slow and wanted something fully
compatible but fast.

This only implements automatic Node version detection, it
doesn't support commands of any sort.

## Usage

Add this to your .zshrc
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

## License
MIT
