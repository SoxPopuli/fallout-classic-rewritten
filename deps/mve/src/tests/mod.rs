#![allow(const_item_mutation)]

use std::fs::File;
use std::path::Path;
use crate::*;

const TEST_DATA: &'static [u8] =
    include_bytes!("../../../../intro.mve");

#[test]
fn parse_test() {
    read_mve(TEST_DATA).unwrap();
}


#[test]
fn header_parse() {
    parse_header(TEST_DATA).unwrap();
}

#[test]
fn chunk_parse() {
    let data: &[u8] = &[ 0x00, 0x00, 0x02, 0x00 ];

    let (_, chunk) = Chunk::parse(data).unwrap();

    assert_eq!(chunk, Chunk { length: 0, typ: ChunkType::InitialiseVideo, data: &[] });
}

#[test]
fn pair_test() {
    let pair = (1, 2);

    let pair2 = pair.map_second(|x| x * 2);

    assert_eq!(pair2, (1, 4))
}
