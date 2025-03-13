### Template Setting

Read in toml in ~/.config/pulith/config.toml or ~/.config/pulith.toml

Profiles with name(otherwise "default"):

```toml
[default]

# support regex for name.
# support string or "y" as true for flags without args.

# define: "replace" = "original"
# profile_name.backend.verb.alias
[default.{backend}.install.flag_alias]
"--path" = "--target-path"

# profile_name.backend.alias
[default.{backend}.flag_alias]
"--slience" =  "--quiet"
"-s" =  "-q"

# profile_name.backend.verb.flag
[default.{backend}.install.flag]
"--verbose" = "y"

# profile_name.backend.flag
[default.{backend}.flag]
"--log-level" = "info"


# parse template to create alias
# { num }: positional args. { 0 } for first args etc..
# { * }: multiple args. should be in the end.
# { --flag }: flags define, single args. { --package -p } for --package and -p
# { --flag * }: flags define, multiple args { --package -p } for --package and -p

[default.alias]
"install_by_id" = "install @{ 0 }:{ 1 } --id { 2 }"

# evaluated in shell(important! install external thing!)
# defined in scipts/insall_by_winget.pwsh
# winget install $0
# defined in scripts/install_by_apt.sh
# sudo apt install $0
[default.alias.script]
"install_by_winget" = "install_by_winget.pwsh"
"install_by_apt" = "install_by_apt.sh"

[config]
editor = "nvim"
```

Template(tera):

```toml
# data
# in data/alice.toml
alice = "Alice"
# in config.toml
name = {{ alice }}

# script
# in scripts/{name}.{ext}
# defined in scipts/insall_by_winget.pwsh
# winget install $0
# defined in scripts/install_by_apt.sh
# sudo apt install $0

# regex
"^(-p|--path)" = "--target-path"

# inheritance
[default]
... defined for default

[profile1]
inheritance = "default"
```
