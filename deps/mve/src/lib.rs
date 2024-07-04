#[cfg(test)]
mod tests;

mod audio;

mod error;
use common::Vec2d;
use error::Error;

mod utils;
use itertools::Itertools;
use utils::MapPair;

mod opcodes;
use opcodes::{Opcode, OpcodeType};

use std::fmt::Debug;

use nom::{
    bytes::complete::take,
    combinator::{iterator, verify},
    IResult,
};
use tap::Pipe;

#[derive(Debug, Default)]
struct VideoSettings {
    /// How long it takes to advance to the next frame
    frame_time: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Default)]
struct Pixel {
    red: u8,
    green: u8,
    blue: u8,
}

/// Represents a chunk of video data
/// Data is fetched and decoded in chunks,
/// to allow for paralell decoding and playing
#[derive(Debug)]
struct DecodedChunk {
    video: common::Vec2d<Pixel>,
    audio: Vec<i32>,
}

fn take_n_bytes_checked<'a, 'b>(
    data: &'a [u8],
    n: usize,
    expected: &'b [u8],
) -> IResult<&'a [u8], &'a [u8]> {
    verify(take(n), |res: &[u8]| *res == *expected)(data)
}

fn parse_header(data: &[u8]) -> IResult<&[u8], ()> {
    take_n_bytes_checked(data, 18usize, "Interplay MVE File".as_bytes())
        .and_then(|(data, _)| {
            take_n_bytes_checked(data, 2usize, &[b'\x1A', b'\0'])
        })
        .and_then(|(data, _)| {
            take_n_bytes_checked(
                data,
                6usize,
                &[0x1a, 0x00, 0x00, 0x01, 0x33, 0x11],
            )
        })
        .map(|(data, _)| (data, ()))
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq)]
enum ChunkType {
    InitialiseAudio,
    AudioOnly,
    InitialiseVideo,
    Video,
    Shutdown,
    End,
}

impl TryFrom<u16> for ChunkType {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::InitialiseAudio),
            0x01 => Ok(Self::AudioOnly),
            0x02 => Ok(Self::InitialiseVideo),
            0x03 => Ok(Self::Video),
            0x04 => Ok(Self::Shutdown),
            0x05 => Ok(Self::End),
            _ => Err(Error::ChunkError(value)),
        }
    }
}

#[derive(PartialEq, Eq)]
struct Chunk<'a> {
    length: u16,
    typ: ChunkType,
    data: &'a [u8],
}

impl std::fmt::Debug for Chunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("length", &self.length)
            .field("typ", &self.typ)
            .finish()
    }
}

impl<'a> Chunk<'a> {
    fn new(
        length: u16,
        typ: u16,
        data: &'a [u8],
    ) -> Result<(&'a [u8], Self), Error> {
        let (rest, data) = take(length as usize)(data)?;
        Chunk {
            length,
            typ: typ.try_into()?,
            data,
        }
        .pipe(|x| (rest, x))
        .pipe(Ok)
    }

    fn parse(data: &'a [u8]) -> IResult<&[u8], Self> {
        let (data, length) = take2(data).map_second(u16::from_le_bytes)?;
        let (data, typ) = take2(data).map_second(u16::from_le_bytes)?;

        Self::new(length, typ, data)
            .map_err(|_| {
                nom::error::Error {
                    input: data,
                    code: nom::error::ErrorKind::TakeUntil,
                }
                .pipe(nom::Err::Failure)
            })?
            .pipe(Ok)
    }
}

fn take_n<const N: usize>(data: &[u8]) -> IResult<&[u8], [u8; N]> {
    let (data, x) = take(N)(data)?;
    let mut buffer = std::mem::MaybeUninit::<[u8; N]>::uninit();

    let output;
    unsafe {
        let ptr = (*buffer.as_mut_ptr()).as_mut_slice();

        for i in 0..N {
            ptr[i] = x[i];
        }
        output = buffer.assume_init();
    }

    Ok((data, output))
}

fn take_i16(data: &[u8]) -> IResult<&[u8], i16> {
    take_n(data).map_second(i16::from_le_bytes)
}

fn take_u16(data: &[u8]) -> IResult<&[u8], u16> {
    take_n(data).map_second(u16::from_le_bytes)
}

fn take2(data: &[u8]) -> IResult<&[u8], [u8; 2]> {
    take_n(data)?.pipe(Ok)
}

fn parse_if<T>(
    predicate: bool,
    data: &[u8],
    parser: impl FnOnce(&[u8]) -> IResult<&[u8], T>,
) -> IResult<&[u8], Option<T>> {
    if predicate {
        parser(data).map_second(Some)
    } else {
        Ok((data, None))
    }
}

fn decode_chunk(
    video_settings: &mut VideoSettings,
    chunk: &Chunk,
) -> Result<(), Error> {
    let opcodes = &mut iterator(chunk.data, Opcode::parse);

    let mut video_buffer = Vec2d::<Pixel>::new(0, 0);

    for op in opcodes {
        match op.typ {
            None => {}

            Some(OpcodeType::InitializeVideoBuffers) => {
                let (data, width) = take_u16(op.data)?;
                let (data, height) = take_u16(data)?;
                let (data, count) = parse_if(op.version >= 1, data, take_u16)?;
                let (data, true_color) =
                    parse_if(op.version >= 2, data, take_u16)?;
            }

            _ => todo!(),
        }
    }

    Ok(())
}

pub fn read_mve(data: &[u8]) -> Result<(), Error> {
    let (data, ()) = parse_header(data).unwrap();

    let mut chunks_iterator = iterator(data, Chunk::parse);
    let chunks = chunks_iterator.collect_vec();

    for chunk in chunks {
        let opcodes = iterator(chunk.data, Opcode::parse).collect_vec();

        println!("{opcodes:?}")
    }

    todo!()
}
