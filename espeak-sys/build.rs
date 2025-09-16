use std::env;

fn main() {
	let target = env::var("TARGET").unwrap();

	match target.as_str() {
		"x86_64-pc-windows-gnu" | "x86_64-pc-windows-msvc" => {
			let current_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

			println!("cargo:rustc-link-search={}/native/x86_64-windows", current_dir);
			println!("cargo:root={}/native/x86_64-windows", current_dir);
			println!("cargo:rustc-link-lib=espeak_lib");
		},
		"x86_64-unknown-linux-gnu" => {
			println!("cargo:rustc-link-lib=espeak");
		},
		_ => panic!("unsupported platform: {}", target),
	}
}
