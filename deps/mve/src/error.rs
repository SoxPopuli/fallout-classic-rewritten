use std::fmt::{Debug, Display, Formatter, write};

use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    FileError,
    ReadError(IoError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}


pub trait FromIoError<T> {
    fn to(self) -> Result<T, Error>;
}
impl<T> FromIoError<T> for Result<T, IoError> {
    fn to(self) -> Result<T, Error> {
        self.map_err(|x| Error::ReadError(x))
    }
}
