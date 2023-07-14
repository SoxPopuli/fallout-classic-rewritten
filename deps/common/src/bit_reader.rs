use std::{ 
    mem::size_of,
    collections::VecDeque,
    num::Wrapping,
};

pub struct BitReader {
    data: VecDeque<u8>,
    bit_index: usize,
}

macro_rules! get_bits {
    ($reader:expr, $bits:expr, $t:ty) => {{
        const BYTE_SIZE: usize = size_of::<$t>();
        const BIT_SIZE: usize = BYTE_SIZE * 8;

        if $bits as usize > BIT_SIZE {
            None
        } else {
            $reader.get_bits_unchecked($bits)
                .map(|x| x as $t)
        }
    }};
}

impl BitReader {
    pub fn new<T: Into<VecDeque<u8>>>(data: T) -> Self {
        Self{
            data: data.into(),
            bit_index: 0,
        }
    }

    pub fn bit_size(&self) -> usize {
        (self.data.len() * 8) - self.bit_index
    }

    pub fn byte_size(&self) -> usize {
        self.data.len()
    }

    pub fn get_bit(&mut self) -> Option<u8> {
        let byte = self.data.front()?;
        let shift = self.bit_index;
        let mask = 1 << shift;

        let mut bit = byte & mask;
        bit = bit >> shift;

        self.bit_index += 1;
        if self.bit_index == 8 {
            self.bit_index = 0;
            self.data.pop_front();
        }
        Some(bit)
    }

    fn build_byte_mask(start: u8, end: u8) -> u8 {
        let mut mask = 0;
        for i in start..end {
            mask |= 1 << i;
        }

        mask
    }

    fn get_bits_unchecked(&mut self, bits: u8) -> Option<u64> {
        let mut result = Wrapping(0u64);
        let mut bits_to_read = bits as usize;
        let mut bits_read = 0;

        while bits_to_read > 0 {
            if self.bit_index == 0 && bits_to_read >= 8 {
                //if can read whole bytes
                let mut byte = (self.data.pop_front()?) as u64;
                byte <<= bits_read;

                result |= byte;
                bits_read += 8;
                bits_to_read -= 8;
            }else {
                let start = self.bit_index;
                let end = (start + bits_to_read).clamp(0, 8);
                let mask = Self::build_byte_mask(start as u8, end as u8);

                let mut val = (self.data.front()? & mask) as u64;
                //shift to byte start
                val >>= start;
                //shift back for output
                val <<= bits_read;

                bits_read += end - start;
                self.bit_index += end - start;
                if self.bit_index >= 8 {
                    self.data.pop_front();
                    self.bit_index = 0;
                }

                //result <<= bits_to_read;
                result = Wrapping(result.0 | val);

                bits_to_read -= end - start;
            } 
        }

        Some(result.0)
    }

    pub fn get_bits_u8(&mut self, bits: u8) -> Option<u8> {
        get_bits!(self, bits, u8)
    }
    pub fn get_bits_u16(&mut self, bits: u8) -> Option<u16> {
        get_bits!(self, bits, u16)
    }
    pub fn get_bits_u32(&mut self, bits: u8) -> Option<u32> {
        get_bits!(self, bits, u32)
    }
    pub fn get_bits_u64(&mut self, bits: u8) -> Option<u64> {
        get_bits!(self, bits, u64)
    }

    pub fn get_bits_i8(&mut self, bits: u8) -> Option<i8> {
        get_bits!(self, bits, i8)
    }
    pub fn get_bits_i16(&mut self, bits: u8) -> Option<i16> {
        get_bits!(self, bits, i16)
    }
    pub fn get_bits_i32(&mut self, bits: u8) -> Option<i32> {
        get_bits!(self, bits, i32)
    }
    pub fn get_bits_i64(&mut self, bits: u8) -> Option<i64> {
        get_bits!(self, bits, i64)
    }

}
