#[cfg(test)]
mod tests;

mod error;
use error::Error;

use std::io::Cursor;

use winnow::prelude::*;
use winnow::token::take;

fn parse_header(data: &mut &[u8]) -> PResult<()> {
    take(18usize)
        .verify(|s: &[u8]| *s == *"Interplay MVE File".as_bytes())
        .parse_next(data)?;

    take(2usize)
        .verify(|s: &[u8]| *s == [b'\x1A', b'\0'])
        .parse_next(data)?;

    take(6usize)
        .verify(|s: &[u8]| *s == [0x1a, 0x00, 0x00, 0x01, 0x33, 0x11])
        .parse_next(data)?;

    Ok(())
}

pub fn read_mve(mut data: &[u8]) -> Result<(), Error> {
    let data = &mut data;
    parse_header(data)
        .map_err(Error::ParseError)?;

    todo!()
}
