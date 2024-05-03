#[cfg(test)]
mod tests;

mod error;
use error::Error;
use winnow::combinator::todo;

use std::io::Cursor;

use nom::{
    bytes::complete::take,
    combinator::{map_res, verify},
    error::ParseError,
    IResult, Parser,
};
use tap::Pipe;

trait MapFirst {
    type A;
    type B;
    type E;
    fn map_first<O>(
        self,
        f: impl FnOnce(Self::A) -> O,
    ) -> Result<(O, Self::B), Self::E>;
}

impl<A, B, E> MapFirst for Result<(A, B), E> {
    type A = A;
    type B = B;
    type E = E;

    fn map_first<O>(self, f: impl FnOnce(Self::A) -> O) -> Result<(O, B), E> {
        match self {
            Ok((a, b)) => Ok((f(a), b)),
            Err(e) => Err(e),
        }
    }
}

trait MapSecond {
    type A;
    type B;
    type E;
    fn map_second<O>(
        self,
        f: impl FnOnce(Self::B) -> O,
    ) -> Result<(Self::A, O), Self::E>;
}

impl<A, B, E> MapSecond for Result<(A, B), E> {
    type A = A;
    type B = B;
    type E = E;

    fn map_second<O>(self, f: impl FnOnce(Self::B) -> O) -> Result<(A, O), E> {
        match self {
            Ok((a, b)) => Ok((a, f(b))),
            Err(e) => Err(e),
        }
    }
}

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

#[derive(Debug)]
struct Chunk {
    length: u16,
    typ: u16,
}

fn take2(data: &[u8]) -> IResult<&[u8], [u8; 2]> {
    let (data, x) = take(2usize)(data)?;
    let mut output: [u8; 2] = 
        unsafe {std::mem::MaybeUninit::zeroed().assume_init()};
    output.copy_from_slice(x);

    Ok((data, output))
}

fn parse_chunk(data: &[u8]) -> IResult<&[u8], Chunk> {
    let (data, length) = 
        take2(data)
        .map_second(u16::from_le_bytes)?;

    let (data, typ) =
        take2(data)
        .map_second(u16::from_le_bytes)?;

    Ok((data, Chunk { length, typ }))
}

pub fn read_mve(data: &[u8]) -> Result<(), Error> {
    let (data, ()) = parse_header(data).unwrap();
    let (data, chunk) = parse_chunk(data).unwrap();

    todo!()
}
