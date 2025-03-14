### Profile Setting

Read in toml in ~/.config/pulith/config.toml or ~/.config/pulith.toml

```toml
editor = "nvim"

# flag replacement/default
# replacement onlt replace flag without knowing args property, it should be ensured by user.
# [{verb}.{flag|*}]
# pat = [{string}]
# arg_pat = { pat = {string}, default = [{string}]/{string} }
# default is optional, flag will be filled if not exist. 
# * is for all if not defined explicitly.

# example:
[flag]

undefined = "ignore" # Options: "ignore", "pass-through", "error"

[flag.install.--path|-p.rye]
pat = ["--force"]
arg_pat = { pat = "--target-path", default = ""}

[flag.install.--path|-p.winget]
arg_pat = { pat = "--location", default = "{...}"}

[flag.install.--path|-p.*]
arg_pat = "--path"

[flag.remove.--force|-f.scoop]
pat = ["--purge"]

[flag.install.--verbose.*]
pat = ["--verbose"]

# parse template to create alias
# { num }: positional args. { 1 } for first args etc..
# { * }: multiple args. should be in the end.
# { --flag }: flags define, single args. { --package -p } for --package and -p
# { --flag * }: flags define, multiple args { --package -p * } for --package and -p
[command]
"install_by_id" = "install @{ --backend }:{ --id } { --custom -c * } { 1 }"
"remove_by_apt" = "remove @apt:--force { 1 }"

# evaluated in shell(important! install external thing!)
# defined in scipts/insall_by_winget.pwsh
# winget install $0
# defined in scripts/install_by_apt.sh
# sudo apt install $0
[command.script]
"install_by_winget" = "install_by_winget.pwsh"
"install_by_apt" = "install_by_apt.sh"
```
