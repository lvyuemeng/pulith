use anyhow::Context;

impl Descriptor {
    pub fn only_bk(self) -> Option<BackendType> {
        if self.name.is_none() && self.ver_key.is_none() {
            self.bk
        } else {
            None
        }
    }
    pub fn only_tool(self) -> Option<(String, VersionKind)> {
        if let (Some(name), Some(ver_key)) = (self.name, self.ver_key) && self.bk.is_none() {
            Some((name, ver_key))
        } else {
            None
        }
    }
}
