//! Template shim binary.
//!
//! Copy this file and customize the resolver type and construction.
//! The key is to implement [`TargetResolver`] and call [`try_run()`] with it.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use pulith_shim::TargetResolver;
use thiserror::Error;

pub fn try_run<R: TargetResolver>(resolver: R) -> Result<ExitStatus, Error> {
    invoke(resolver, env::args())
}

pub fn invoke<R, I, S>(resolver: R, args: I) -> Result<ExitStatus, Error>
where
    R: TargetResolver,
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let (command, forwarded_args) = parse_invoke(args)?;
    let target = resolve_target(&resolver, &command)?;
    exec(&target, &forwarded_args)
}

fn parse_invoke<I, S>(args: I) -> Result<(String, Vec<String>), Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);

    let _shim_exe = args.next();
    let command = args.next().ok_or(Error::MissingCommand)?;
    let forwarded = args.collect();
    Ok((command, forwarded))
}

fn resolve_target<R: TargetResolver>(resolver: &R, command: &str) -> Result<PathBuf, Error> {
    let target = resolver
        .resolve(command)
        .ok_or_else(|| Error::CommandNotFound(command.to_string()))?;
    validate_target(&target)?;
    Ok(target)
}

fn validate_target(target: &Path) -> Result<(), Error> {
    if !target.exists() {
        return Err(Error::TargetNotFound(target.to_path_buf()));
    }
    Ok(())
}

fn exec(target: &Path, args: &[String]) -> Result<ExitStatus, Error> {
    Command::new(target)
        .args(args)
        .status()
        .map_err(Error::ProcessFailed)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no command specified")]
    MissingCommand,
    #[error("command not found: '{0}'")]
    CommandNotFound(String),
    #[error("target not found: {0}")]
    TargetNotFound(PathBuf),
    #[error("process failed: {0}")]
    ProcessFailed(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Resolver(Option<PathBuf>);

    impl TargetResolver for Resolver {
        fn resolve(&self, _command: &str) -> Option<PathBuf> {
            self.0.clone()
        }
    }

    #[test]
    fn parse_invoke_extracts_command_and_forwarded_args() {
        let parsed = parse_invoke(["shim.exe", "tool", "--flag", "value"]).unwrap();
        assert_eq!(parsed.0, "tool");
        assert_eq!(parsed.1, vec!["--flag", "value"]);
    }

    #[test]
    fn parse_invoke_rejects_missing_command() {
        assert!(matches!(
            parse_invoke(["shim.exe"]),
            Err(Error::MissingCommand)
        ));
    }

    #[test]
    fn resolve_target_rejects_missing_target() {
        assert!(matches!(
            resolve_target(&Resolver(None), "tool"),
            Err(Error::CommandNotFound(command)) if command == "tool"
        ));
    }

    #[test]
    fn validate_target_rejects_nonexistent_path() {
        assert!(matches!(
            validate_target(Path::new("/definitely/missing/target")),
            Err(Error::TargetNotFound(_))
        ));
    }

    #[test]
    fn resolve_target_re_resolves_each_invocation() {
        use std::sync::Mutex;

        struct SwitchingResolver {
            values: Mutex<Vec<PathBuf>>,
        }

        impl TargetResolver for SwitchingResolver {
            fn resolve(&self, _command: &str) -> Option<PathBuf> {
                self.values.lock().unwrap().pop()
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("runtime-a");
        let second = temp.path().join("runtime-b");
        std::fs::write(&first, b"a").unwrap();
        std::fs::write(&second, b"b").unwrap();

        let resolver = SwitchingResolver {
            values: Mutex::new(vec![first.clone(), second.clone()]),
        };

        let resolved_1 = resolve_target(&resolver, "tool").unwrap();
        let resolved_2 = resolve_target(&resolver, "tool").unwrap();

        assert_eq!(resolved_1, second);
        assert_eq!(resolved_2, first);
    }
}
