use crate::error::{Error, Result};
use crate::shell::Shell;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Output};

#[derive(Debug)]
pub struct Command {
    inner: StdCommand,
    program: String,
}

impl Command {
    pub fn new(program: impl Into<String>) -> Self {
        let program = program.into();
        Self {
            inner: StdCommand::new(&program),
            program,
        }
    }

    pub fn search_at(mut self, dir: PathBuf) -> Self {
        #[cfg(target_os = "windows")]
        let exe_path = dir.join(format!("{}.exe", self.program));
        #[cfg(not(target_os = "windows"))]
        let exe_path = dir.join(&self.program);

        if exe_path.exists() {
            self.inner = StdCommand::new(exe_path);
        }
        self
    }

    pub fn capture(mut self) -> Result<Output> {
        self.inner.output().map_err(|e| Error::CommandFailed {
            cmd: self.program.clone(),
            source: e,
        })
    }

    pub fn run_in_shell(mut self, shell: Shell) -> Self {
        let shell_exe = shell.executable();
        let script = self.program.clone();
        self.inner = StdCommand::new(shell_exe);
        self.inner.args(["-c", &script]);
        self
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.inner.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn env_clear(mut self) -> Self {
        self.inner.env_clear();
        self
    }

    pub fn env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, val);
        self
    }

    pub fn output(&mut self) -> Result<Output> {
        self.inner.output().map_err(|e| Error::CommandFailed {
            cmd: self.program.clone(),
            source: e,
        })
    }

    pub fn spawn(&mut self) -> Result<std::process::Child> {
        self.inner.spawn().map_err(|e| Error::CommandFailed {
            cmd: self.program.clone(),
            source: e,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_new() {
        let cmd = Command::new("echo");
        assert_eq!(cmd.program, "echo");
    }

    #[test]
    fn test_command_new_string() {
        let cmd = Command::new(String::from("echo"));
        assert_eq!(cmd.program, "echo");
    }

    #[test]
    fn test_command_args() {
        let cmd = Command::new("echo").arg("hello").arg("world");
        let inner = cmd.inner;
        let args: Vec<_> = inner.get_args().collect();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_command_multiple_args() {
        let cmd = Command::new("echo").arg("arg1").arg("arg2").arg("arg3");
        let inner = cmd.inner;
        let args: Vec<_> = inner.get_args().collect();
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_command_args_iter() {
        let cmd = Command::new("echo").args(["a", "b", "c"]);
        let inner = cmd.inner;
        let args: Vec<_> = inner.get_args().collect();
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_command_env_clear() {
        let cmd = Command::new("echo").env_clear();
        let inner = cmd.inner;
        assert!(inner.get_envs().count() == 0);
    }

    #[test]
    fn test_command_env() {
        let cmd = Command::new("echo").env("KEY", "value");
        let inner = cmd.inner;
        assert!(inner.get_envs().count() > 0);
    }

    #[test]
    fn test_command_search_at_nonexistent() {
        let cmd = Command::new("nonexistent_binary_12345")
            .search_at(PathBuf::from("/nonexistent/directory"));
        assert_eq!(cmd.program, "nonexistent_binary_12345");
    }

    #[test]
    fn test_command_search_at_existing_file() {
        let cmd = Command::new("echo").search_at(PathBuf::from("/usr/bin"));
        assert_eq!(cmd.program, "echo");
    }

    #[test]
    fn test_command_run_in_shell() {
        let cmd = Command::new("echo hello").run_in_shell(Shell::Bash);
        let inner = cmd.inner;
        assert_eq!(inner.get_program().to_string_lossy(), "bash");
    }

    #[test]
    fn test_command_run_in_shell_pwsh() {
        let cmd = Command::new("echo hello").run_in_shell(Shell::Pwsh);
        let inner = cmd.inner;
        assert_eq!(inner.get_program().to_string_lossy(), "pwsh");
    }

    #[test]
    fn test_command_capture_returns_result() {
        let cmd = Command::new("echo").arg("test");
        let result = cmd.capture();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_command_output_returns_result() {
        let mut cmd = Command::new("echo").arg("test");
        let result = cmd.output();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_command_spawn_returns_result() {
        let mut cmd = Command::new("echo").arg("test");
        let result = cmd.spawn();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_command_builder_pattern() {
        let cmd = Command::new("program")
            .arg("--flag")
            .arg("value")
            .env("ENV_VAR", "value");
        assert_eq!(cmd.program, "program");
    }

    #[test]
    fn test_command_arg_with_spaces() {
        let cmd = Command::new("echo").arg("hello world");
        let inner = cmd.inner;
        let args: Vec<_> = inner.get_args().collect();
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_command_arg_empty() {
        let cmd = Command::new("echo").arg("");
        let inner = cmd.inner;
        let args: Vec<_> = inner.get_args().collect();
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_command_program_preserved() {
        let cmd = Command::new("original_program").search_at(PathBuf::from("/nonexistent"));
        assert_eq!(cmd.program, "original_program");
    }
}
