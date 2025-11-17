use crate::backend::Backend;
use statum::{machine, state};
use std::marker::PhantomData;

#[state]
enum InstallState {
    Init,
    // Native install
    NativeInstall,
    // Manual install
    Pre,
    Install,
    Post,
    // Complete
    Complete,
}

#[machine]
#[derive(Debug)]
pub struct InstallerInner<S: InstallState> {}

pub struct Installer<T:Backend,S:InstallState> {
    inner: InstallerInner<S>,
    _bk: PhantomData<T>,
}
