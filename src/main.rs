use reg::SaveGuard;

mod backend;
mod cli;
mod env;
mod reg;
mod tool;
mod utils;

fn main() {
	// auto save on exit
	let _sg = SaveGuard::new();
}