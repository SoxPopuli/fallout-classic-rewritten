use std::env;
use std::path::PathBuf;

fn main() {
    let files = [
        //"acmtool.c",
        "decode.c",
        "util.c",
    ];

    let mut compiler = cc::Build::new();
    compiler.files(
        files.map(|f| format!("libacm/src/{}", f))
    )
    .include("libacm/src/")
    .warnings(false)
    .compile("libacm");


    let bindings = bindgen::Builder::default()
        .header("libacm/src/libacm.h")
        .generate()
        .unwrap();

    let out_dir = PathBuf::from( env::var("OUT_DIR").unwrap() );
    bindings.write_to_file(out_dir.join("bindings.rs"))
        .unwrap();
}
