use std::fs::File;
use std::path::Path;
use crate::*;

use crate::IntoDeltaIterator;

const TEST_DATA: &'static [u8] =
    include_bytes!("../../../../reference/f2/master/art/cuts/afailed.mve");

#[test]
fn no_errors() {
    read_mve(TEST_DATA).unwrap();
}

#[test]
fn delta_iter_test() {
    let values = [0, 1, 2, 3, -3, -2, -1];
    let deltas: Vec<_> = 
        values
            .into_iter()
            .deltas()
            .unwrap()
            .collect();

    assert_eq!(&deltas, &[0, 1, 3, 6, 3, 1, 0]);
}