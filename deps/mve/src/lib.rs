pub mod error;

#[cfg(test)]
mod tests;

pub use error::Error;

use common::{ read_num, read_bytes };
use std::{
    io::{Cursor, Read}, 
    ops::{ Add, Sub },
    cell::Cell,
};
use crate::Error::FileError;

use itertools::Itertools;

macro_rules! read_mve {
    ($data:expr, $t:ty) => {{
        read_num!($data, $t, le)
    }};
}

struct DeltaIterator<T, I> where T: Add<Output = T> + Copy, I: Iterator<Item = T> {
    value: Option<T>,
    iter: I,
}
impl<T, I> DeltaIterator<T, I> where T: Add<Output = T> + Copy, I: Iterator<Item = T> {
    fn get_value(&self) -> Option<T> { self.value }
}
impl<T, I> Iterator for DeltaIterator<T, I> where T: Add<Output = T> + Copy, I: Iterator<Item = T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.value {
            Some(v) => {
                let next = self.iter.next()?;
                self.value = Some(next + v);

                self.value
            },

            None => {
                self.value = Some(self.iter.next()?);
                self.value
            }
        }
    }
}

trait IntoDeltaIterator<T> : Iterator<Item = T> where T: Add<Output = T> + Copy {
    type Output;
    fn deltas(self) -> Option<Self::Output>;
    fn deltas_from_initial(self, initial: T) -> Option<Self::Output>;
}
impl<T: Add<Output = T> + Copy, I: Iterator<Item = T>> IntoDeltaIterator<T> for I {
    type Output = DeltaIterator<T, I>;

    fn deltas(mut self) -> Option<Self::Output> {
        Some(DeltaIterator{ value: None, iter: self })
    }

    fn deltas_from_initial(self, initial: T) -> Option<Self::Output> {
        Some(DeltaIterator { value: Some(initial), iter: self })
    }
}

fn uncompress_audio(data: &[i8]) -> Option<Vec<i16>> {
    const DELTA_CODINGS: [i16; 256] = 
        [
             0,      1,      2,      3,      4,      5,      6,      7,      8,      9,     10,     11,     12,     13,     14,     15,
            16,     17,     18,     19,     20,     21,     22,     23,     24,     25,     26,     27,     28,     29,     30,     31,
            32,     33,     34,     35,     36,     37,     38,     39,     40,     41,     42,     43,     47,     51,     56,     61,
            66,     72,     79,     86,     94,    102,    112,    122,    133,    145,    158,    173,    189,    206,    225,    245,
            267,    292,    318,    348,    379,    414,    452,    493,    538,    587,    640,    699,    763,    832,    908,    991,
            1081,   1180,   1288,   1405,   1534,   1673,   1826,   1993,   2175,   2373,   2590,   2826,   3084,   3365,   3672,   4008,
            4373,   4772,   5208,   5683,   6202,   6767,   7385,   8059,   8794,   9597,  10472,  11428,  12471,  13609,  14851,  16206,
            17685,  19298,  21060,  22981,  25078,  27367,  29864,  32589, -29973, -26728, -23186, -19322, -15105, -10503,  -5481,     -1,
                1,      1,   5481,  10503,  15105,  19322,  23186,  26728,  29973, -32589, -29864, -27367, -25078, -22981, -21060, -19298,
        -17685, -16206, -14851, -13609, -12471, -11428, -10472,  -9597,  -8794,  -8059,  -7385,  -6767,  -6202,  -5683,  -5208,  -4772,
            -4373,  -4008,  -3672,  -3365,  -3084,  -2826,  -2590,  -2373,  -2175,  -1993,  -1826,  -1673,  -1534,  -1405,  -1288,  -1180,
            -1081,   -991,   -908,   -832,   -763,   -699,   -640,   -587,   -538,   -493,   -452,   -414,   -379,   -348,   -318,   -292,
            -267,   -245,   -225,   -206,   -189,   -173,   -158,   -145,   -133,   -122,   -112,   -102,    -94,    -86,    -79,    -72,
            -66,    -61,    -56,    -51,    -47,    -43,    -42,    -41,    -40,    -39,    -38,    -37,    -36,    -35,    -34,    -33,
            -32,    -31,    -30,    -29,    -28,    -27,    -26,    -25,    -24,    -23,    -22,    -21,    -20,    -19,    -18,    -17,
            -16,    -15,    -14,    -13,    -12,    -11,    -10,     -9,     -8,     -7,     -6,     -5,     -4,     -3,     -2,     -1
        ];
    

    Some(
        data.into_iter()
        .map(|i| *i as i16)
        .deltas()?
        .map(|i| DELTA_CODINGS[i as usize])
        .collect()
    )
}

#[derive(Debug)]
struct Header {
    file_type: String,
    magic_bytes: [u16; 3],
}

fn unwrap_inner<T, E>(r: Result< Result<T, E>, E >) -> Result<T, E> {
    match r {
        Ok( Ok(o) ) => Ok(o),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(e),
    }
}

fn get_remaining<T>(c: &Cursor<T>) -> &[u8] where T: AsRef<[u8]> {
    let pos = c.position() as usize;
    &c.get_ref().as_ref()[pos..]
}

fn unsigned_to_signed(data: &[u8]) -> Vec<i8> {
    let ptr = data.as_ptr() as *mut i8;
    unsafe{ Vec::from_raw_parts(ptr, data.len(), data.len()) }
}

#[derive(Debug)]
struct Color {
    red: u8,
    green:u8,
    blue: u8,
}

// Opcodes
#[derive(Debug)] enum AudioChannels { Mono, Stereo }
#[derive(Debug)] enum AudioChannelWidth { Bit8, Bit16 }
#[derive(Debug)] enum AudioCompression { Uncompressed, Compressed }

#[derive(Debug)]
struct AudioFlags {
    channels: AudioChannels,
    channel_width: AudioChannelWidth,
    compression: AudioCompression,
}

#[derive(Debug)]
enum InitAudioBuffers {
    V0{ 
        channels: AudioChannels,
        channel_width: AudioChannelWidth,
        sample_rate: u16,
        min_buf_len: u16
    },
    V1{ 
        channels: AudioChannels,
        channel_width: AudioChannelWidth,
        compression: AudioCompression,
        sample_rate: u16,
        min_buf_len: u32,
    },
}

#[derive(Debug)]
enum InitVideoBuffers {
    V0{ width: u16, height: u16 },
    V1{ width: u16, height: u16, count: u16 },
    V2{ width: u16, height: u16, count: u16, true_color: u16 },
}

#[derive(Debug)]
enum SendBufferToDisplay {
    V0{ palette_start: u16, palette_count: u16 },
    V1{ palette_start: u16, palette_count: u16, unknown1: u16 },
}

#[repr(u16)]
enum LanguageFlags {
    English = 0
}

/// Contains data about the audio
/// Data storage depends on previously encountered audio flags
/// 
/// * `seq_index` - Sequential number of audio frames
/// * `stream_mask` - bit mask used to indicate language, e.g. bit 0 = English
/// * `stream_len` - length of data stream
/// * `data` - audio data, if Compressed data is delta encoded
#[derive(Debug)]
enum AudioFrame {
    Data{ seq_index: u16, stream_mask: u16, stream_len: u16, data: Vec<i8> },
    Silence{ seq_index: u16, stream_mask: u16, stream_len: u16 },
}
impl AudioFrame {
    fn get_samples(&self, flags: &AudioFlags) -> Vec<i16> {
        match self {
            Self::Data { seq_index, stream_mask, stream_len, data } => {
                todo!()
            }
            
            Self::Silence { seq_index, stream_mask, stream_len } => {
                //stream mask in silence ops are inverted?
                let stream_mask = !stream_mask;

                todo!()
            },
        }
    }
}

struct OpcodeIterator<'a> {
    length: usize,
    data: Cursor<&'a [u8]>
}
impl<'a> Iterator for OpcodeIterator<'a> {
    type Item = Opcode;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.data.position() as usize) < self.length {
            let len = read_mve!(self.data, u16)?;
            let type_ = read_mve!(self.data, u8)?;
            let ver = read_mve!(self.data, u8)?;

            let mut op_data = vec![0u8; len as usize];
            self.data.read_exact(&mut op_data).ok()?;

            Opcode::from_data(type_, ver, &op_data)
        } else {
            None
        }
    }
}

struct ChunkIterator<'a> {
    length: usize,
    data: Cursor<&'a [u8]>,
}
impl<'a> Iterator for ChunkIterator<'a> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        
        if (self.data.position() as usize) < self.length {
            let len = read_mve!(self.data, u16)?;
            let chunk_type = ChunkType::from_int(read_mve!(self.data, u16)? as i32)?;

            let mut buf = vec![0u8; len as usize];
            self.data.read_exact(&mut buf).ok()?;

            let opcode_iter = OpcodeIterator{ length: len as usize, data: Cursor::new(&buf) };

            Some(Chunk{ chunk_type, opcodes: opcode_iter.collect() })
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum Opcode {
    EndOfStream,
    EndOfChunk,
    CreateTimer { rate: u32, subdivision: u16 },
    InitAudioBuffers(InitAudioBuffers),
    StartStopAudio,
    InitVideoBuffers(InitVideoBuffers),
    SendBufferToDisplay(SendBufferToDisplay),
    AudioFrame(AudioFrame),
    InitVideoMode{ width: u16, height: u16, flags: u16 },
    CreateGradient,
    SetPalette{ palette_start: u16, palette_count: u16, data: Vec<Color> },
    SetPaletteCompressed(Vec<u8>),
    SetDecodingMap(Vec<u8>),
    VideoData(Vec<u8>),
    Unknown(u8), //store type value for ease of debugging
}
impl Opcode {
    fn read_opcodes(data: &[u8]) -> Option<Vec<Opcode>> {
        let length = data.len();
        let mut data = Cursor::new(data);

        let mut opcodes = vec![];
        while (data.position() as usize) < length {
            let len = read_mve!(data, u32)?;
            let type_ = read_mve!(data, u8)?;
            let ver = read_mve!(data, u8)?;

            let mut op_data = vec![0u8; len as usize];
            data.read_exact(&mut op_data).ok()?;

            opcodes.push(Self::from_data(type_, ver, &op_data)?);
        }

        Some(opcodes)
    }

    fn from_data(type_: u8, ver: u8, data: &[u8]) -> Option<Self> {
        let mut data = Cursor::new(data);

        match type_ {
            0x00 => Some(Self::EndOfStream),
            0x01 => Some(Self::EndOfChunk),
            0x02 => Some(Self::CreateTimer { 
                rate: read_mve!(data, u32)?,
                subdivision: read_mve!(data, u16)?,
            }),

            0x03 => Self::read_init_audio_buffers(&mut data, ver),
            0x04 => Some(Self::StartStopAudio),
            0x05 => Self::read_init_video_buffers(&mut data, ver),
            0x07 => {
                let palette_start = read_mve!(data, u16)?;
                let palette_count = read_mve!(data, u16)?;

                let version = if ver == 0 {
                    SendBufferToDisplay::V0 { palette_start, palette_count }
                } else {
                    SendBufferToDisplay::V1 { palette_start, palette_count, unknown1: read_mve!(data, u16)? }
                };

                Some(Self::SendBufferToDisplay(version))
            }

            t @ 0x08 ..= 0x09 => {
                let seq_index = read_mve!(data, u16)?;
                let stream_mask = read_mve!(data, u16)?;
                let stream_len = read_mve!(data, u16)?;
                
                let version = if t == 0x08 {
                    let data = unsigned_to_signed(get_remaining(&data));
                    AudioFrame::Data { seq_index, stream_mask, stream_len, data }
                } else {
                    AudioFrame::Silence { seq_index, stream_mask, stream_len }
                };

                Some(Self::AudioFrame(version))
            },

            0x0A => Some(Self::InitVideoMode { 
                width: read_mve!(data, u16)?,
                height: read_mve!(data, u16)?,
                flags: read_mve!(data, u16)?,
            }),

            0x0B => Some(Self::CreateGradient),

            0x0C => {
                let palette_start = read_mve!(data, u16)?;
                let palette_count = read_mve!(data, u16)?;
                let data = 
                    get_remaining(&data).iter()
                        .tuples::<(_, _, _)>()
                        .map(|(r, g, b)| Color{ red: *r, green: *g, blue: *b })
                        .collect();
                    
                Some(Self::SetPalette { palette_start, palette_count, data })
            },

            0x0D => Some(Self::SetPaletteCompressed( Vec::from(data.into_inner()) )),

            0x0F => Some(Self::SetDecodingMap( Vec::from(data.into_inner()) )),

            0x11 => Some(Self::VideoData( Vec::from(data.into_inner()) )),

            t @ (0x06 | 0x0E | 0x010 | 0x12 | 0x14 | 0x15) => Some(Self::Unknown(t)),
            _ => None,
        }
    }

    fn read_init_audio_buffers(data: &mut Cursor<&[u8]>, ver: u8) -> Option<Opcode> {
        let _unknown = read_mve!(data, u16)?;
        let flags = read_mve!(data, u16)?;
        let sample_rate = read_mve!(data, u16)?;

        let channels = 
            if (flags & 0b1) == 0 { AudioChannels::Mono }
            else { AudioChannels::Stereo };

        let channel_width =
            if (flags & 0b10) == 0 { AudioChannelWidth::Bit8 }
            else { AudioChannelWidth::Bit16 };

        let version = if ver == 0 {
            let min_buf_len = read_mve!(data, u16)?;
            InitAudioBuffers::V0 { 
                channels,
                channel_width,
                sample_rate,
                min_buf_len,
            }
        } else {
            let compression =
                if (flags & 0b100) == 0 { AudioCompression::Uncompressed }
                else { AudioCompression::Compressed };

            let min_buf_len = read_mve!(data, u32)?;
            InitAudioBuffers::V1 { 
                channels,
                channel_width,
                compression,
                sample_rate,
                min_buf_len,
            }
        };

        Some(Self::InitAudioBuffers(version))
    }

    fn read_init_video_buffers(data: &mut Cursor<&[u8]>, ver: u8) -> Option<Opcode> {
        let width = read_mve!(data, u16)?;
        let height = read_mve!(data, u16)?;
        let version = if ver == 0 {
            InitVideoBuffers::V0 { width, height }
        } else {
            let count = read_mve!(data, u16)?;

            if ver == 1 {
                InitVideoBuffers::V1 { width, height, count }
            } else {
                let true_color = read_mve!(data, u16)?;
                InitVideoBuffers::V2 { width, height, count, true_color }
            }
        };

        Some(Self::InitVideoBuffers(version))
    }
}


// End Opcodes

#[derive(Debug)]
enum ChunkType {
    InitAudio,
    AudioOnly,
    InitVideo,
    VideoChunk,
    ShutdownChunk,
    EndChunk,
}
impl ChunkType {
    fn from_int(i: i32) -> Option<Self> {
        match i {
            0 => Some(Self::InitAudio),
            1 => Some(Self::AudioOnly),
            2 => Some(Self::InitVideo),
            3 => Some(Self::VideoChunk),
            4 => Some(Self::ShutdownChunk),
            5 => Some(Self::EndChunk),

            _ => None,
        }
    }
}

#[derive(Debug)]
struct Chunk {
    chunk_type: ChunkType,
    opcodes: Vec<Opcode>,
}
impl Chunk {
    fn from_data(data: &[u8]) -> Option<(Self, Vec<u8>)> {
        if data.len() == 0 { return None; }

        let mut data = Cursor::new(data);

        let len = read_mve!(data, u16)?;
        let chunk_type = ChunkType::from_int(read_mve!(data, u16)? as i32)?;

        // if len == 0 { return None; }

        let data: &[u8] = get_remaining(&data);
        // let data = data.into_inner();
        if len as usize > data.len() { return None }

        let (chunk, rest) = data.split_at(len as usize);

        let iter = OpcodeIterator{ length: chunk.len(), data: Cursor::new(chunk) };
        let opcodes = iter.collect::<Vec<_>>();

        Some((Self{
            chunk_type,
            opcodes,
        }, Vec::from(rest)))
    }
}

fn read_chunk_rec(mut acc: Vec<Chunk>, data: &[u8]) -> Vec<Chunk> {
    match Chunk::from_data(data) {
        None => acc,
        Some((c, d)) => read_chunk_rec({acc.push(c); acc}, &d ),
    }
}


pub fn read_mve(data: &[u8]) -> Result<(), Error> {
    // let mut data = std::io::BufReader::new(data);
    let mut data = Cursor::new(data);

    let file_type = read_bytes!(data, 20).map_err(|_| FileError)?;
    let magic_bytes = (0..3)
        .filter_map(|_| read_bytes!(data, 2).ok())
        .map(|bytes| u16::from_le_bytes(bytes))
        .collect::<Vec<_>>();

    if file_type != "Interplay MVE File\x1a\0".as_bytes() ||
        magic_bytes != [ 0x001a, 0x0100, 0x1133 ] {
        return Err(FileError);
    }

    let rest = get_remaining(&data);
    let chunks = ChunkIterator {
        length: rest.len(),
        data: Cursor::new(rest)
    };

    let _c = chunks.collect::<Vec<_>>();

    todo!()
}