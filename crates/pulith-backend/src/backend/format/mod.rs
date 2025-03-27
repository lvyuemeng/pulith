use pulith_core::ui::table::Formatter;
use tabled::{Table, Tabled};

use crate::backend::Backend;

pub trait ParseFormat<B: Backend, O:Clone+Tabled> {
    type Parsed: IntoIterator<Item = O>;
    type Err;
    fn parse(bk: &B) -> Result<Self::Parsed, Self::Err>;
	fn format(data:Self::Parsed,cfg:Formatter) -> Table {
		let table = cfg.build(data);
		table
	}
}



