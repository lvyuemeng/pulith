trait Backend {
    fn metadata() -> Metadata;
    fn runtime_data() -> RuntimeData;
    fn cmd() -> Option<impl Iterator<Item = Command>> {
        None
    }

    // Tool ops
    fn add();
    fn use_ver();
    fn remove();
    fn list();
    fn update();
    fn search();
}

pub struct CheckReg;
pub struct Ops;
pub struct UpdateReg;
