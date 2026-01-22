# pulith-shim

Platform-independent executable shim mechanism. Mechanism-only; policy delegated to `TargetResolver`.

## API

```rust
pub trait TargetResolver: Send + Sync {
    fn resolve(&self, command: &str) -> Option<PathBuf>;
}

// Composables
PairResolver::new(primary, fallback)      // Try primary, then fallback
TripleResolver::new(a, b, c)              // Chain three resolvers
```

## Shim Binary

```rust
// pulith-shim-bin provides generic shim template
fn main() {
    let args = env::args().collect();
    let resolver = MyResolver::new();
    let target = resolver.resolve(&args[0])?;

    std::process::Command::new(&target)
        .args(&args[1..])
        .status()?;
}
```

## Example

```rust
struct VersionResolver { versions_dir: PathBuf, version_var: String }

impl TargetResolver for VersionResolver {
    fn resolve(&self, cmd: &str) -> Option<PathBuf> {
        let v = std::env::var(&self.version_var).ok()?;
        let p = self.versions_dir.join(&v).join("bin").join(cmd);
        p.exists().then_some(p)
    }
}
```

## Dependencies

```
thiserror
```

## Composition Patterns

```rust
// Fallback
PairResolver::new(env_resolver, default_resolver);

// Chain
TripleResolver::new(env, file, default);
```
