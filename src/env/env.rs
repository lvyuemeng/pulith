use crate::env::SystemInfo;
use anyhow::{bail, Result};
use std::env::{self, split_paths, SplitPaths};
use std::ffi::OsStr;
use std::process::Command;
pub struct EnvExec;

impl EnvExec {
    pub fn paths() -> Result<SplitPaths> {
        let path_var = if cfg!(target_os = "windows") {
            "Path"
        } else {
            "PATH"
        };
        
        match env::var_os(path_var) {
            Some(paths) => Ok(split_paths(&paths).),
            None => bail!("{} cannot be found", path_var),
        }
    }
    pub fn local_shell<K, V>(exports: impl IntoIterator<Item = (K, V)>) -> Result<()>
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let shell = SystemInfo::shell_exec()?;
        let mut cmd = Command::new(shell).envs(exports).spawn()?;

        cmd.wait()?;

        Ok(())
    }
}
