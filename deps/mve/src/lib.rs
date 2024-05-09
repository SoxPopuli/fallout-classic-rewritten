#[cfg(test)]
mod tests;

mod error;
use error::Error;

mod utils;
use itertools::Itertools;
use utils::MapPair;

use std::fmt::Debug;

use nom::{
    bytes::complete::take,
    combinator::{iterator, verify},
    IResult,
};
use tap::Pipe;

fn take_n_bytes_checked<'a, 'b>(
    data: &'a [u8],
    n: usize,
    expected: &'b [u8],
) -> IResult<&'a [u8], &'a [u8]> {
    verify(take(n), |res: &[u8]| *res == *expected)(data)
}

fn parse_header(data: &[u8]) -> IResult<&[u8], ()> {
    let (data, _) =
        take_n_bytes_checked(data, 18usize, "Interplay MVE File".as_bytes())?;

    let (data, _) = take_n_bytes_checked(data, 2usize, &[b'\x1A', b'\0'])?;

    let (data, _) = take_n_bytes_checked(
        data,
        6usize,
        &[0x1a, 0x00, 0x00, 0x01, 0x33, 0x11],
    )?;

    Ok((data, ()))
}

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
    body: &'a [u8],
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
        let (data, body) = take(length as usize)(data)?;
        Chunk {
            length,
            typ: typ.try_into()?,
            body,
        }
        .pipe(|x| (data, x))
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

fn take2(data: &[u8]) -> IResult<&[u8], [u8; 2]> {
    take_n(data)?.pipe(Ok)
}

fn parse_chunk(data: &[u8]) -> IResult<&[u8], Chunk> {
    let (data, length) = take2(data).map_second(u16::from_le_bytes)?;

    let (data, typ) = take2(data).map_second(u16::from_le_bytes)?;

    Chunk::new(length, typ, data)
        .map_err(|_| {
            nom::error::Error {
                input: data,
                code: nom::error::ErrorKind::TakeUntil,
            }
            .pipe(nom::Err::Failure)
        })?
        .pipe(Ok)
}

pub fn read_mve(data: &[u8]) -> Result<(), Error> {
    let (data, ()) = parse_header(data).unwrap();

    let chunks = iterator(data, parse_chunk).collect_vec();

    println!("{:#?}", chunks);

    //let (data, chunk) = parse_chunk(data).unwrap();

    todo!()
}
