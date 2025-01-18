use once_cell::sync::Lazy;

type Cache<T> = Lazy<T>;

trait Reg {
    type ctx;
    type config;
    type out;

    fn get_or_init() -> Result<ctx>;
    fn list() -> out;
}
