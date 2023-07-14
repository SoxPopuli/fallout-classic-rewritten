use std::error::Error;
use std::fmt;
use FrmError::*;

pub type Result<T> = std::result::Result<T, FrmError>;

#[derive(Debug, Clone)]
pub enum FrmError {
    InvalidSig,
    ReadError,
    SizeMismatch
}

impl fmt::Display for FrmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidSig => write!(f, "Invalid Signature"),
            ReadError => write!(f, "Error reading file"),
            SizeMismatch => write!(f, "Frame size does not match width x height"),
        }
    }
}

impl Error for FrmError {}
