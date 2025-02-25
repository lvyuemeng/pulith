use crate::env::SystemInfo;
use anyhow::Result;
use std::env::{self, split_paths};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, CommandEnvs};
pub struct EnvExec;

impl EnvExec {
    pub fn paths() -> Option<Vec<PathBuf>> {
        let path_var = if cfg!(target_os = "windows") {
            "Path"
        } else {
            "PATH"
        };

        env::var_os(path_var).map(|paths| split_paths(&paths).collect())
    }

    pub fn local_shell<K, V>(exports: impl IntoIterator<Item = (K, V)>) -> Result<()>
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let shell = SystemInfo::which_shell().unwrap();
        let mut cmd = Command::new(shell).envs(exports).spawn()?;

        cmd.wait()?;

        Ok(())
    }
}
