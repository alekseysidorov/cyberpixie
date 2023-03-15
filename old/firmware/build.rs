use std::{env, fs, path::PathBuf};

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn main() {
    // Put the memory definitions somewhere the linker can find it
    let out_dir = out_dir();
    println!("cargo:rustc-link-search={}", out_dir.display());

    fs::copy("memory-cb.x", out_dir.join("memory-cb.x")).unwrap();
    println!("cargo:rerun-if-changed=memory-cb.x");
}
