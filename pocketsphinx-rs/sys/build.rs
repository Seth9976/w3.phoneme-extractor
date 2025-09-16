use std::env;
use std::path::PathBuf;

use cmake::Config;

fn main() {
    let target = env::var("TARGET").unwrap();

    match target.as_str() {
        "x86_64-pc-windows-gnu" | "x86_64-pc-windows-msvc" => {
            let current_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

            println!(
                "cargo:rustc-link-search={}/native/x86_64-windows",
                current_dir
            );
            println!("cargo:root={}/native/x86_64-windows", current_dir);
            println!("cargo:rustc-link-lib=static=pocketsphinx");
        }
        "x86_64-unknown-linux-gnu" => {
            let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

            // build libpocketsphinx.a
            let dst = Config::new("pocketsphinx")
                .define("CMAKE_INSTALL_PREFIX", &out_path)
                .build();

            println!("cargo:rustc-link-search=native={}/lib", dst.display());
            println!("cargo:rustc-link-lib=static=pocketsphinx");

            println!("cargo:rustc-link-search={}", out_path.display());
            println!("cargo:rerun-if-changed=wrapper.h");

            // Generate bindings
            let bindings = bindgen::Builder::default()
                .header("wrapper.h")
                .clang_arg(format!("-I{}/include", dst.display()))
                .generate()
                .expect("Unable to generate bindings");

            // Write the bindings to the $OUT_DIR/bindings.rs file.
            bindings
                .write_to_file(out_path.join("bindings.rs"))
                .expect("Couldn't write bindings!");
        }
        _ => panic!("unsupported platform: {}", target),
    }
}
