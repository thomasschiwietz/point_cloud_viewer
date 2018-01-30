extern crate gl_generator;
extern crate protoc_rust;

use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("bindings.rs")).unwrap();

    Registry::new(Api::Gl, (3, 2), Profile::Core, Fallbacks::All, [])
        .write_bindings(StructGenerator, &mut file)
        .unwrap();

    println!("cargo:rerun-if-changed=src/proto.proto");

    let out_dir = env::var("OUT_DIR").unwrap();
    protoc_rust::run(protoc_rust::Args {
        out_dir: &out_dir,
        input: &["src/proto.proto"],
        includes: &[],
    }).expect("protoc");

    // Work around
    // https://github.com/stepancheg/rust-protobuf/issues/117
    // https://github.com/rust-lang/rust/issues/18810.
    // We open the file, add 'mod proto { }' around the contents and write it back. This allows us
    // to include! the file in lib.rs and have a proper proto module.
    let proto_path = Path::new(&out_dir).join("proto.rs");
    let mut contents = String::new();
    File::open(&proto_path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    let new_contents = format!("pub mod proto {{\n{}\n}}", contents);

    File::create(&proto_path)
        .unwrap()
        .write_all(new_contents.as_bytes())
        .unwrap();
}
