use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use image::io::Reader;
use quote::quote;

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn convert_image() {
    let name = "images/cat_dark.png";

    let raw_img_def = {
        let image = Reader::open(name).unwrap().decode().unwrap().to_rgb8();
        let buf_items = image.pixels().map(|x| {
            let r = x[0];
            let g = x[1];
            let b = x[2];

            quote! {
                RGB8 { r: #r, g: #g, b: #b }
            }
        });
        let image_len = (image.height() * image.width()) as usize;

        let body = quote! {
            // This file is generated automatically, do not edit it!

            use smart_leds::RGB8;

            pub const DATA: [RGB8; #image_len] = [
                #(#buf_items),*
            ];
        };
        body.to_string()
    };

    let mut out_file = File::create(out_dir().join("raw_image.rs")).unwrap();
    out_file.by_ref().write_all(raw_img_def.as_bytes()).unwrap();

    println!("cargo:rerun-if-changed={}", name);
}

fn main() {
    // Put the memory definitions somewhere the linker can find it
    let out_dir = out_dir();
    println!("cargo:rustc-link-search={}", out_dir.display());

    fs::copy("memory-cb.x", out_dir.join("memory-cb.x")).unwrap();
    println!("cargo:rerun-if-changed=memory-cb.x");

    convert_image();
}
