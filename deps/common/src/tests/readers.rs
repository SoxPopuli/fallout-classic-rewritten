use std::error::Error;

use crate::readers::*;

#[test]
fn type_reader_test() -> Result<(), Box<dyn Error>> {
    fn make_data() -> &'static [u8] {
        &[
            1u8,
            2u8,
            3u8,
            4u8,
        ]
    }

    let expected_le = 
        4 << 24 |
        3 << 16 |
        2 << 8  |
        1 << 0;

    let expected_be = 
        1 << 24 |
        2 << 16 |
        3 << 8  |
        4 << 0;

    let le: i32 = read_type(ReadMode::LE, &mut make_data())?;
    let be: i32 = read_type(ReadMode::BE, &mut make_data())?;

    assert_eq!(le,  expected_le);
    assert_eq!(be, expected_be);


    let mut b: &[u8] = &[0u8];
    let actual: bool = read_type(ReadMode::NE, &mut b)?;
    assert_eq!(
        actual,
        false
    );

    let mut b: &[u8] = &[5];
    let actual: bool = read_type(ReadMode::NE, &mut b)?;
    assert_eq!(
        actual,
        true
    );

    Ok(())
}
