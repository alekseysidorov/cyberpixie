use std::{env, fs};
use std::path::PathBuf;

fn main() {
    // Put the memory definitions somewhere the linker can find it
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out_dir.display());

    fs::copy("memory-cb.x", out_dir.join("memory-cb.x")).unwrap();
    println!("cargo:rerun-if-changed=memory-cb.x");
}
