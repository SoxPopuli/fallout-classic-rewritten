pub mod color;
pub use color::Color;

use common::Stream;

use std::{
    mem::size_of
};

const COLOR_COUNT: usize = 256;
const CONVERSION_TABLE_COUNT: usize = 32768;

#[derive(Debug)]
pub struct PalFile {
    pub colors: [Color; COLOR_COUNT],
    pub conversion_table: [u8; CONVERSION_TABLE_COUNT],
}

impl PalFile {
    pub fn open(file: &mut dyn Stream) -> Option<Self> {
        let mut colors = [Color::default(); COLOR_COUNT];
        let mut conversion_table = [0u8; CONVERSION_TABLE_COUNT];

        let colors_size = (3 * size_of::<u8>()) * COLOR_COUNT;
        let conversion_table_size = size_of::<u8>() * CONVERSION_TABLE_COUNT;
        let palette_size = colors_size + conversion_table_size;

        let mut read_buf = vec![0u8; palette_size];
        file.read(&mut read_buf).ok()?;

        let mut i = 0;
        let mut color_index = 0;
        let mut conv_table_index = 0;
        while i < palette_size {
            if i < COLOR_COUNT * 3 {
                let mut color_values = [
                    read_buf[i+0],
                    read_buf[i+1],
                    read_buf[i+2],
                ];

                for c in color_values.iter_mut() {
                    if *c < 64 && i > 0 {
                        *c *= 4;
                    }
                }

                colors[color_index] = color_values.into();
                i += 2;
                color_index += 1;
            } else {
                conversion_table[conv_table_index] = read_buf[i];
                conv_table_index += 1;
            }

            i += 1;
        }

        Some(Self{
            colors,
            conversion_table,
        })
    }
}
