#![allow(const_item_mutation)]

use std::fs::File;
use std::path::Path;
use crate::*;

const TEST_DATA: &'static [u8] =
    include_bytes!("../../../../intro.mve");

//#[test]
//fn no_errors() {
//    read_mve(TEST_DATA).unwrap();
//}


#[test]
fn header_parse() {
    parse_header(TEST_DATA).unwrap();
}

#[test]
fn chunk_parse() {
    let data: &[u8] = &[ 0x24, 0x03, 0x02, 0x00 ];

    let (_, chunk) = parse_chunk(data).unwrap();

    println!("{:#?}", chunk);
    todo!()
}
