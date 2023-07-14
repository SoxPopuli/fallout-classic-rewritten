use std::error::Error;
use std::fmt::{self, write};
use DatError::*;

pub type Result<T> = std::result::Result<T, DatError>;

#[derive(Debug, Clone)]
pub enum DatError {
    InvalidSig,
    ReadError,
    LZSSError,
    TreeError,
    TreeNodeError,
}

impl fmt::Display for DatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidSig => write!(f, "Invalid Signature"),
            ReadError => write!(f, "Error reading file"),
            LZSSError => write!(f, "Error unpacking LZSS"),
            TreeError => write!(f, "Error creating tree"),
            TreeNodeError => write!(f, "Incorrect Node type"),
        }
    }
}

impl Error for DatError {}
