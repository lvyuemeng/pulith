use statum::{machine, state};
use std::marker::PhantomData;

#[state]
enum InstallState {
    Init,
    // native package manager installation.
    NativeInstall(String),
    // manual installation.
    Download(Url),
    Install(PathBuf),
    Complete,
}

struct InstallSetting {
    dry_run: bool,
}

#[machine]
#[derive(Debug)]
pub struct Installer<S: InstallState,T:Backend> {
    setting: InstallSetting,
    _bk: PhantomData<T>,
}
