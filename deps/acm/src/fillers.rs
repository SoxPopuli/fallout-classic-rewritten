use std::{collections::VecDeque, ops::Range};
use common::{
    BitReader,
    Vec2d,
};
use super::AcmError;

const BUFFER_MIDDLE: usize = 0x8_000;
const MAP_1BIT: [usize; 2] = [ BUFFER_MIDDLE - 1, BUFFER_MIDDLE + 1 ];
const MAP_2BIT_NEAR: [usize; 4] = [ BUFFER_MIDDLE-2, BUFFER_MIDDLE-1, BUFFER_MIDDLE+1, BUFFER_MIDDLE+2 ];
const MAP_2BIT_FAR: [usize; 4] = [ BUFFER_MIDDLE-3, BUFFER_MIDDLE-2, BUFFER_MIDDLE+2, BUFFER_MIDDLE+3 ];
const MAP_3BIT: [usize; 8] = [ BUFFER_MIDDLE-4, BUFFER_MIDDLE-3, BUFFER_MIDDLE-2, BUFFER_MIDDLE-1, BUFFER_MIDDLE+1, BUFFER_MIDDLE+2, BUFFER_MIDDLE+3, BUFFER_MIDDLE+4, ];

fn get_index_1(bit: u8) -> usize {
    MAP_1BIT[bit as usize]

    /*
    match bit {
        0 => BUFFER_MIDDLE - 1,
        1 => BUFFER_MIDDLE + 1,
        _ => panic!("bit index: {} out of range", bit),
    }
    */
}

fn get_index_2_near(bits: u8) -> usize {
    MAP_2BIT_NEAR[bits as usize]

    /*
    match bits {
        0b00 => BUFFER_MIDDLE - 2,
        0b10 => BUFFER_MIDDLE - 1,
        0b01 => BUFFER_MIDDLE + 1,
        0b11 => BUFFER_MIDDLE + 2,
        _ => panic!("bit index: {} out of range", bits),
    }
    */
}

fn get_index_2_far(bits: u8) -> usize {
    MAP_2BIT_FAR[bits as usize]

    /*
    match bits {
        0b00 => BUFFER_MIDDLE - 3,
        0b10 => BUFFER_MIDDLE - 2,
        0b01 => BUFFER_MIDDLE + 2,
        0b11 => BUFFER_MIDDLE + 3,
        _ => panic!("bit index: {} out of range", bits),
    }
    */
}

fn get_index_3(bits: u8) -> usize {
    MAP_3BIT[bits as usize]

    /*
    match bits {
        0b000 => BUFFER_MIDDLE - 4,
        0b100 => BUFFER_MIDDLE - 3,
        0b010 => BUFFER_MIDDLE - 2,
        0b110 => BUFFER_MIDDLE - 1,
        0b001 => BUFFER_MIDDLE + 1,
        0b101 => BUFFER_MIDDLE + 2,
        0b011 => BUFFER_MIDDLE + 3,
        0b111 => BUFFER_MIDDLE + 4,
        _ => panic!("bit index: {} out of range", bits),
    }
    */
}

pub struct FillerArgs<'a> {
    pub reader: &'a mut BitReader,
    pub packed_block: &'a mut Vec2d<i32>,
    pub amp_buffer: &'a mut [i32],

    pub index: usize,
    pub column: usize,
}

impl<'a> FillerArgs<'a> {
    fn set_in_column(&mut self, row: usize, value: i32) {
        self.packed_block.insert(self.column, row, value);
        //self.packed_block.insert(row, self.column, value);
    }

    fn set_by_index(&mut self, row: usize, index: usize) {
        let amp = self.amp_buffer[index];
        self.set_in_column(row, amp);
    }

    fn block_rows(&self) -> Range<usize> {
        0 .. self.packed_block.height()
    }
}

pub enum Fillers {
    Zero,
    Ret0,
    Linear,
    K13,
    K12,
    T15,
    K24,
    K23,
    T27,
    K35,
    K34,
    K45,
    K44,
    T37,
}

impl Fillers {
    pub fn fill(&self, args: FillerArgs) -> Result<(), AcmError> {
        match self {
            Self::Zero    =>  fill_zero(args),
            Self::Ret0    =>  fill_ret0(args),
            Self::Linear  =>  fill_linear(args),
            Self::K13     =>  fill_k13(args),
            Self::K12     =>  fill_k12(args),
            Self::T15     =>  fill_t15(args),
            Self::K24     =>  fill_k24(args),
            Self::K23     =>  fill_k23(args),
            Self::T27     =>  fill_t27(args),
            Self::K35     =>  fill_k35(args),
            Self::K34     =>  fill_k34(args),
            Self::K45     =>  fill_k45(args),
            Self::K44     =>  fill_k44(args),
            Self::T37     =>  fill_t37(args),
        }
    }
}

impl From<u8> for Fillers {
    fn from(index: u8) -> Self {
        match index {
            0        =>  Self::Zero,
            1..=2    =>  Self::Ret0,
            3..=16   =>  Self::Linear,
            17       =>  Self::K13,
            18       =>  Self::K12,
            19       =>  Self::T15,
            20       =>  Self::K24,
            21       =>  Self::K23,
            22       =>  Self::T27,
            23       =>  Self::K35,
            24       =>  Self::K34,
            25       =>  Self::Ret0,
            26       =>  Self::K45,
            27       =>  Self::K44,
            28       =>  Self::Ret0,
            29       =>  Self::T37,
            30..=31  =>  Self::Ret0,

            _ => panic!("index out of range"),
        }
    }
}

fn fill_zero(mut args: FillerArgs) -> Result<(), AcmError> {
    for i in args.block_rows() {
        args.set_in_column(i, 0);
    }
        Ok(())
}

fn fill_ret0(_args: FillerArgs) -> Result<(), AcmError> {
    Err(AcmError::CorruptBlock)

    /*
    let width = args.packed_block.width();
    let height = args.packed_block.height();

    *args.packed_block = Vec2d::new(width, height);

    Ok(())
    */
}

fn fill_linear(mut args: FillerArgs) -> Result<(), AcmError> {
    for i in args.block_rows() {
        let val = args.reader.get_bits_i64(args.index as u8).ok_or(AcmError::FillError)?;
        let idx = 0x8_000 + val;
        let amp = args.amp_buffer[idx as usize];
        
        args.set_in_column(i, amp);
    }

    Ok(())
}

macro_rules! get_bit {
    ($reader:expr) => {{
        $reader.get_bit().ok_or(AcmError::BitError)
    }};
}

fn fill_k13(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 { // 0
            args.set_in_column(i, 0);
            if let Some(next_i) = iter.next() {
                args.set_in_column(next_i, 0);
            }
            continue;
        } else if get_bit!(args.reader)? == 0 { // 1, 0
            args.set_in_column(i, 0);
        } else {
            let bit = get_bit!(args.reader)?;
            let index = get_index_1(bit);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k12(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 { // 0
            args.set_in_column(i, 0);
        } else { // 1, X
            let bit = get_bit!(args.reader)?;
            let index = get_index_1(bit);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k24(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 { // 0 
            args.set_in_column(i, 0);
            if let Some(next_i) = iter.next() {
                args.set_in_column(next_i, 0);
            }
        } else if get_bit!(args.reader)? == 0 { // 1, 0,
            args.set_in_column(i, 0);
        } else { // 1, 1, X, Y
            let bits = args.reader.get_bits_u8(2).ok_or(AcmError::FillError)?;
            let index = get_index_2_near(bits);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k23(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 {
            args.set_in_column(i, 0);
        } else {
            let bits = args.reader.get_bits_u8(2).ok_or(AcmError::FillError)?;
            let index = get_index_2_near(bits);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k35(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 { // 0
            args.set_in_column(i, 0);
            if let Some(next_i) = iter.next() {
                args.set_in_column(next_i, 0);
            }
        } else if get_bit!(args.reader)? == 0 { // 1, 0
            args.set_in_column(i, 0);
        } else if get_bit!(args.reader)? == 0 { // 1, 1, 0, X
            let bit = get_bit!(args.reader)?;
            let index = get_index_1(bit);
            args.set_by_index(i, index);
        } else { // 1, 1, 1, XX
            let bits = args.reader.get_bits_u8(2).ok_or(AcmError::FillError)?;
            let index = get_index_2_far(bits);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k34(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 { // 0
            args.set_in_column(i, 0);
        } else if get_bit!(args.reader)? == 0 {
            let index = match get_bit!(args.reader)? { // 1, 0, X
                0 => BUFFER_MIDDLE - 1,
                1 => BUFFER_MIDDLE + 1,
                _ => panic!(),
            };
            args.set_by_index(i, index);
        } else { // 1, 1, XX
            let bits = args.reader.get_bits_u8(2).ok_or(AcmError::FillError)?;
            let index = get_index_2_far(bits);
            args.set_by_index(i, index);
        }
    }
    
    Ok(())
}

fn fill_k45(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 {
            args.set_in_column(i, 0);
            if let Some(next_i) = iter.next() {
                args.set_in_column(next_i, 0);
            }
        } else if get_bit!(args.reader)? == 0 {
            args.set_in_column(i, 0);
        } else {
            let bits = args.reader.get_bits_u8(3).ok_or(AcmError::FillError)?;
            let index = get_index_3(bits);
            args.set_by_index(i, index);
        }
    }

    Ok(())
}

fn fill_k44(mut args: FillerArgs) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        if get_bit!(args.reader)? == 0 {
            args.set_in_column(i, 0);
        } else {
            let bits = args.reader.get_bits_u8(3).ok_or(AcmError::FillError)?;
            let index = get_index_3(bits);
            args.set_by_index(i, index);
        }
    }


    Ok(())
}

fn fill_txx(mut args: FillerArgs, bits: u8, base: i32, times: usize) -> Result<(), AcmError> {
    let mut iter = args.block_rows();
    while let Some(i) = iter.next() {
        let bits = args.reader.get_bits_u8(bits).ok_or(AcmError::FillError)?;
        let mut digits = change_base(bits as i32, base);

        let digit = digits.pop_back().unwrap_or(0);
        let index = BUFFER_MIDDLE + digit as usize;
        args.set_by_index(i, index);

        for _ in 0 .. times-1 {
            let digit = digits.pop_back().unwrap_or(0);
            let index = BUFFER_MIDDLE + digit as usize;
            if let Some(next_i) = iter.next() {
                args.set_by_index(next_i, index);
            }
        }
    }

    Ok(())
}


fn fill_t15(args: FillerArgs) -> Result<(), AcmError> {
    fill_txx(args, 5, 3, 3)
}

fn fill_t27(args: FillerArgs) -> Result<(), AcmError> {
    fill_txx(args, 7, 5, 3)
}

fn fill_t37(args: FillerArgs) -> Result<(), AcmError> {
    fill_txx(args, 7, 11, 2)
}

fn change_base(mut num: i32, base: i32) -> VecDeque<i32> {
    if num == 0 || base == 0 {
        return VecDeque::from([0]);
    }

    let mut digits = VecDeque::new();
    let mut div;
    let mut rem;

    loop {
        div = num / base;
        rem = num % base;
        digits.push_back(rem);
        num = div;

        if div == 0 {
            break;
        }
    }

    digits
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn change_base(num: i32, base: i32) -> VecDeque<i32> {
        super::change_base(num, base)
    }

}
