use std::{
    error::Error,
    fmt::{ Result, Display }
};

#[derive(Debug)]
pub enum AcmError {
    StreamError,
    SigError,
    BitError,
    FillError,
    CorruptBlock,
    WriteError,
}

impl Display for AcmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        use AcmError::*;
        match self {
            StreamError => write!(f, "Error reading from stream"),
            SigError => write!(f, "Invalid file signature"),
            BitError => write!(f, "Error reading bits"),
            FillError => write!(f, "Error filling block"),
            CorruptBlock => write!(f, "Acm block may be corrupt"),
            WriteError => write!(f, "Failed to write data"),
        }
    }
}

impl Error for AcmError {}

