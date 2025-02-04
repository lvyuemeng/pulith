use statum::{machine, state};

pub mod bk;

#[state]
enum InstallState {
    Init,
    // native package manager installation.
    NativeInstall(String),
    // script, compressed, exe.
    Download(Url),
    Extract(PathBuf),
    Install(PathBuf),
    Complete,
}

struct InstallSetting {
    dry_run: bool,
}

#[machine]
#[derive(Debug)]
pub struct Installer<S: InstallState> {
    setting: InstallSetting,
}

impl Installer<Init> {
    pub fn use_native(self, name: String) -> Installer<NativeInstall> {
        self.transition_with(name)
    }
    pub fn use_download(self, url: impl Into<Url>) -> Installer<Download> {
        self.transition_with(url)
    }
}

impl Installer<NativeInstall> {
    pub fn install(self) -> Result<()> {
        todo!()       
    }
}

impl Installer<Download> {
    pub fn download() -> Result<Installer<Extract>> {
        todo!()
    }
}

impl Installer<Extract> {
    pub fn extract() -> Result<Installer<Install>> {
        todo!()
    }
}

impl Installer<Install> {
    pub fn install() -> Result<Installer<Complete>> {
        todo!()
    }
}

impl Installer<Complete> {
    pub fn complete() -> Result<()> {
        // clean temp, 
        todo!()
    }
}


