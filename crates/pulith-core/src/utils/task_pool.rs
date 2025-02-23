use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static POOL: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_io().build().unwrap()
});
