use crate::{
    bit_reader::BitReader,
};


#[test]
fn get_bit_test() {
    let byte1 = 0b1001_0110;
    let byte2 = 0b0101_0001;

    let mut reader = BitReader::new([
        byte1,
        byte2
    ]);

    let mut gb = || {
        reader.get_bit().unwrap()
    };

    //byte1
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 1);
    assert_eq!(gb(), 1);
    assert_eq!(gb(), 0);

    assert_eq!(gb(), 1);
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 1);

    //byte2
    assert_eq!(gb(), 1);
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 0);

    assert_eq!(gb(), 1);
    assert_eq!(gb(), 0);
    assert_eq!(gb(), 1);
    assert_eq!(gb(), 0);
}


#[test]
fn get_bits_test() {
    let bits = [
        25, 2, 128, 204,
        8, 255, 2, 5,
    ];

    let mut reader = BitReader::new(bits);

    let power = reader.get_bits_u8(4).unwrap();
    assert_eq!(power, 9);

    let value = reader.get_bits_u16(16).unwrap();
    assert_eq!(value, 33);
}
