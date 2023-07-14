#[cfg(test)]
mod tests;

pub mod bitmap;
pub use bitmap::{ Bitmap, Color };

pub mod error;
use error::FrmError;
use FrmError::*;

use common::{ Stream, read_num };
use pal::PalFile;

use std::rc::Rc;

#[derive(Debug, Default, Clone, Copy)]
pub struct PixelShift {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Default, Clone)]
pub struct Frame {
    pub width: u16,
    pub height: u16,
    pub size: u32,

    pub shift: PixelShift,
    pub color_index: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct FrmFile {
    pub fps: u16,
    pub action_frame: u16,
    pub frames_per_direction: u16,
    pub shifts: [PixelShift; 6],
    pub frame_offsets: [u32; 6],

    pub frames: Vec<Frame>,
}

macro_rules! read_frm {
    ($file:expr, $t: ty) => {{
        read_num!($file, $t, be).ok_or(ReadError)
    }};
}

impl FrmFile {
    pub fn open(file: &mut dyn Stream) -> Result<Self, FrmError> {
        let mut this = Self::default();

        let ver = read_frm!(file, u32)?;
        if ver != 0x04 {
            return Err(InvalidSig);
        }

        this.fps = read_frm!(file, u16)?;
        this.action_frame = read_frm!(file, u16)?;
        this.frames_per_direction = read_frm!(file, u16)?;
        
        for i in 0..6 {
            this.shifts[i].x = read_frm!(file, i16)?;
        }
        for i in 0..6 {
            this.shifts[i].y = read_frm!(file, i16)?;
        }
        for i in 0..6 {
            this.frame_offsets[i] = read_frm!(file, u32)?;
        }

        let data_size = read_frm!(file, u32)?;

        let frame_start = file.stream_position().map_err(|_| ReadError)?;
        let frame_end = frame_start + data_size as u64;

        while file.stream_position().map_err(|_| ReadError)? < frame_end {
            let mut frame = Frame::default();

            frame.width = read_frm!(file, u16)?;
            frame.height = read_frm!(file, u16)?;

            frame.size = read_frm!(file, u32)?;

            if frame.width as u32 * frame.height as u32 != frame.size {
                return Err(SizeMismatch);
            }

            frame.shift.x = read_frm!(file, i16)?;
            frame.shift.y = read_frm!(file, i16)?;

            let mut buf = vec![0u8; frame.size as usize];
            file.read(&mut buf).map_err(|_| ReadError)?;
            frame.color_index = buf;

            this.frames.push(frame);
        }

        Ok(this)
    }

    //returns none if out of bounds
    fn apply_pixel_shift(index: usize, frame: &Frame) -> Option<usize> {
        let shift_x = frame.shift.x as isize;
        let shift_y = frame.shift.y as isize;

        if shift_x == 0 && shift_y == 0 {
            //no shift necessary
            return Some(index);
        }

        let width = frame.width as isize;
        let height = frame.height as isize;
        let size = width * height;

        let x = index as isize % width;
        let y = index as isize / width;

        let shifted_x = x + shift_x;
        let shifted_y = y + shift_y;

        let is_in_range = (shifted_x >= 0 && shifted_x < width) &&
            (shifted_y >= 0 && shifted_y < height);

        if !is_in_range {
            return None;
        }

        let shifted_index = shifted_x + shifted_y * width;

        if shifted_index < 0 || shifted_index >= size {
            return None;
        }

        Some(shifted_index as usize)
    }

    pub fn decode(&self, palette: &PalFile) -> Vec<Bitmap> {
        let mut images = Vec::new();

        for f in self.frames.iter() {
            let mut b = Bitmap{
                width: f.width as u64,
                height: f.height as u64,
                pixels: vec![Color::new(0, 0, 0, 0); f.size as usize],
            };

            for i in 0..b.pixels.len() {
                let palette_index = f.color_index[i];

                if let Some(index) = Self::apply_pixel_shift(i, &f) {
                    if palette_index > 0 {
                        let rgb = palette.colors[palette_index as usize];
                        let c = Color::new(rgb.red, rgb.green, rgb.blue, 255);
                        b.pixels[index] = c;
                    }
                }
            }

            images.push(b);
        }

        images
    }
}
