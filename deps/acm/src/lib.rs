
#[cfg(test)]
mod tests;

mod libacm;

pub mod error;
mod fillers;

use self::error::AcmError;

use common::Stream;
use std::{
    mem::size_of,
    io::Write,
};

type SampleType = i16;

#[derive(Debug, Default, Clone)]
pub struct Acm {
    pub channels: u32,
    pub sample_rate: u32,

    pub samples: Vec<SampleType>,
}

impl Acm {
    pub fn open(mut stream: impl Stream + Sized, force_channels: Option<i32>) -> Result<Acm, AcmError> {
        let data = stream.to_cursor().expect("failed to read data");
        let acm = libacm::read_data(data, force_channels)?;

        Ok(acm)
    }

    pub fn write_to_wav(&self) -> Result<Vec<u8>, AcmError> {
        let mut output = Vec::new();
        let header = WavHeader::new(&self);

        output.extend_from_slice(&header.write()?);
        for s in &self.samples {
            let bytes = s.to_le_bytes();
            output.write(&bytes).map_err(|_| AcmError::WriteError)?;
        }

        Ok(output)
    }
}

#[repr(C)]
pub struct WavHeader {
    pub riff: [u8; 4],
    pub size: u32,
    pub wave: [u8; 4],
    pub fmt: [u8; 4],
    pub wave_size: u32,
    pub wave_type: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub bytes_per_sec: u32,
    pub alignment: u16,
    pub bits_per_sample: u16,
    pub data_header: [u8; 4],
    pub data_size: u32,
}

macro_rules! write_array {
    ($a: expr, $out: expr) => {{
        $out.write($a).map_err(|_| AcmError::WriteError)
    }};
}

macro_rules! write_int {
    ($i: expr, $out: expr) => {{
        let bytes = $i.to_le_bytes();
        $out.write(&bytes).map_err(|_| AcmError::WriteError)
    }};
}

impl WavHeader {
    pub fn new(acm: &Acm) -> Self {
        let word_len = size_of::<SampleType>();
        let data_size = acm.samples.len() * word_len;
        let bps = acm.sample_rate as u32 * acm.channels as u32 * word_len as u32;
        let bits = word_len * 8;
        let alignment = bits * acm.channels as usize * 8;

        WavHeader {
            riff: ['R', 'I', 'F', 'F'].map(|x| x as u8),
            size: (size_of::<WavHeader>() + data_size - 8) as u32,
            wave: ['W', 'A', 'V', 'E'].map(|x| x as u8),
            fmt:  ['f', 'm', 't', ' '].map(|x| x as u8),
            wave_size: 16,
            wave_type: 0x01,
            channels: acm.channels as u16,
            sample_rate: acm.sample_rate,
            bytes_per_sec: bps,
            alignment: alignment as u16,
            bits_per_sample: bits as u16,
            data_header: ['d', 'a', 't', 'a'].map(|x| x as u8),
            data_size: data_size as u32,
        }
    }

    pub fn write(&self) -> Result<Vec<u8>, AcmError> {
        trait Integer {}
        impl Integer for u32 {}

        let mut output = Vec::new();

        write_array!(&self.riff, output)?;
        write_int!(self.size, output)?;
        write_array!(&self.wave, output)?;
        write_array!(&self.fmt, output)?;
        write_int!(self.wave_size, output)?;
        write_int!(self.wave_type, output)?;
        write_int!(self.channels, output)?;
        write_int!(self.sample_rate, output)?;
        write_int!(self.bytes_per_sec, output)?;
        write_int!(self.alignment, output)?;
        write_int!(self.bits_per_sample, output)?;
        write_array!(&self.data_header, output)?;
        write_int!(self.data_size, output)?;

        Ok(output)
    }
}


