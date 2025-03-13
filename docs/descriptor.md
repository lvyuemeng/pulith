## Descriptor

```toml
<descriptor> = "[backend-spec]"
<descriptor> = "[^backend-spec]:[^flag-spec] [*package-spec]"

backend-spec = "@[backend]"
package-spec = "[package-name]@[version]"
flag-spec = "[flag-list]"

# define: verb <descriptor>
# example:
"install @rye"
"install @rye:-f python@3.12"
"install @winget rage@1.10.0"
"install '@scoop:--no-cache --global' rage"
"install rage restic"
"remove @apt:-f rage restic"
"search rage"
"search @apt rage"

<verb> = "install/search/remove/update"
install/remove/update = "[*package-spec]"
search = "[package-spec]"
```

