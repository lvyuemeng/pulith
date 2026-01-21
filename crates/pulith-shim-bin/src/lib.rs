//! Template shim binary.
//!
//! # Usage
//!
//! Copy this file and customize the resolver type and construction.
//! The key is to implement [`TargetResolver`] and call [`run()`] with it.

use pulith_shim::TargetResolver;
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

pub fn try_run<R: TargetResolver>(r: R) -> Result<ExitStatus, Error> {
    let (command, forwarded_args) = parse_invoke()?;

    let target = r
        .resolve(&command)
        .ok_or_else(|| Error::CommandNotFound(command))?;

    validate(&target)?;
    exec(&target, forwarded_args)
}

fn parse_invoke() -> Result<(String, Vec<String>), Error> {
    let mut args = env::args();

    let _shim_exe = args.next();
    let command = args.next().ok_or(Error::MissingCommand)?.to_string();
    let forward = args.collect();
    Ok((command, forward))
}

fn validate(target: &PathBuf) -> Result<(), Error> {
    if !target.exists() {
        return Err(Error::TargetNotFound(target.clone()));
    }
    Ok(())
}

fn exec(target: &PathBuf, args: Vec<String>) -> Result<ExitStatus, Error> {
    Command::new(target)
        .args(args)
        .status()
        .map_err(|e| Error::ProcessFailed(e))
}

#[derive(Debug)]
pub enum Error {
    MissingCommand,
    CommandNotFound(String),
    TargetNotFound(PathBuf),
    ProcessFailed(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingCommand => write!(f, "no command specified"),
            Error::CommandNotFound(cmd) => write!(f, "command not found: '{cmd}'"),
            Error::TargetNotFound(path) => write!(f, "target not found: {}", path.display()),
            Error::ProcessFailed(err) => write!(f, "process failed: {err}"),
        }
    }
}

impl std::error::Error for Error {}