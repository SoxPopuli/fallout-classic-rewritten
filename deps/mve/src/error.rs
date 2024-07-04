use std::fmt::{Debug, Display, Formatter};
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    FileError,
    ReadError(IoError),
    ChunkError(u16),
    StreamMaskError(u16),
    ParseIncomplete,
    ParseError {
        input: String,
        code: nom::error::ErrorKind,
    },
}

pub(crate) type NomResult<I, O> = Result<(I, O), Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError => write!(f, "FileError"),
            Self::ReadError(e) => write!(f, "ReadError: {e:?}"),
            Self::ChunkError(code) => {
                write!(f, "ChunkError: unexpected chunk type {code}")
            }
            Self::StreamMaskError(code) => {
                write!(f, "StreamMaskError: unexpected mask value {code}")
            }
            Self::ParseIncomplete => write!(f, "ParseIncomplete"),
            e @ Self::ParseError { .. } => write!(f, "{e:#?}"),
        }
    }
}

impl std::error::Error for Error {}

type NomError<T> = nom::Err<nom::error::Error<T>>;

impl<T: Debug> From<NomError<T>> for Error {
    fn from(value: NomError<T>) -> Self {
        match value {
            nom::Err::Incomplete(_) => Error::ParseIncomplete,
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                let input = format!("{:?}", e.input);
                Error::ParseError {
                    input,
                    code: e.code,
                }
            }
        }
    }
}

pub trait FromIoError<T> {
    fn to(self) -> Result<T, Error>;
}
impl<T> FromIoError<T> for Result<T, IoError> {
    fn to(self) -> Result<T, Error> {
        self.map_err(|x| Error::ReadError(x))
    }
}
