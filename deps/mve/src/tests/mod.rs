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
    parse_header(&mut TEST_DATA).unwrap();
}
