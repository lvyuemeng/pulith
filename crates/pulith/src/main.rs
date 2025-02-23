use pulith_core::reg::SaveGuard;

mod env;
mod cli;
mod reg;
fn main() {
	// auto save on exit
	let _sg = SaveGuard::new();
}