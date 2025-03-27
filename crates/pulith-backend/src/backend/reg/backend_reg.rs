use std::{collections::HashMap, ops::Deref};

use pulith_core::reg::RegLoader;

use crate::backend::{BackendType, Snap};

// pub type BackendRegLoader = RegLoader<BackendStorage>;
// type BackendStorage = HashMap<BackendType, Snap>;

// pub struct BackendRegAPI;

// impl BackendRegAPI {
//     pub fn peek_snap<'a,'b>(reg: &'a BackendRegLoader, bk: &'b BackendType) -> Option<&'a Snap> {
//         reg.deref().get(bk)
//     }
    
//     pub fn get_snap(reg: &BackendRegLoader,bk: &BackendType) -> Option<Snap> {
//         reg.deref().get(bk).cloned()
//     }
// }

