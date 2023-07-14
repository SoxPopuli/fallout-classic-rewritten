mod files;
use files::FileType;

use common::Stream;
use std::{
    io::{
        stdin,
        Read,
        Cursor,
        BufWriter,
        stdout,
        Write
    },
    fs::File,
    path::Path
};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    ///Input file path
    #[clap(value_parser)]
    input: String,

    ///Palette file path
    #[clap(short, long, value_parser)]
    palette: Option<String>,

    ///Inspect file instead of opening
    #[clap(short, long, value_parser)]
    inspect: bool,
}


fn main() {
    let args = Args::parse();


    let file_path = Path::new(&args.input);
    let file = File::open(file_path).unwrap();

    if let Some(ext) = file_path.extension() {
        let ext = ext.to_str().unwrap();
        let file_type = FileType::new(ext).expect("file extension not recognized");

        if args.inspect {
            file_type.inspect(file);
        } else {
            file_type.open(file, args.palette);
        }
    }
}
