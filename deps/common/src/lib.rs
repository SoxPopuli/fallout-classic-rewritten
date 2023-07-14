#[cfg(test)]
mod tests;

pub mod bit_reader;
pub mod vec_2d;
pub mod stream;

pub use self::{
    bit_reader::BitReader,
    vec_2d::Vec2d,
    stream::Stream,
};

#[macro_export]
macro_rules! read_num {
    ($reader:expr, $itype:ty, be) => {
        {
            let mut buf = [0u8; std::mem::size_of::<$itype>()];
            $reader.read(&mut buf)
                .ok()
                .and_then(|_| { Some(<$itype>::from_be_bytes(buf)) })
        }
    };
    ($reader:expr, $itype:ty, le) => {
        {
            let mut buf = [0u8; std::mem::size_of::<$itype>()];
            $reader.read(&mut buf)
                .ok()
                .and_then(|_| { Some(<$itype>::from_le_bytes(buf)) })
        }
    };
    ($reader:expr, $itype:ty) => {
        {
            let mut buf = [0u8; std::mem::size_of::<$itype>()];
            $reader.read(&mut buf)
                .ok()
                .and_then(|_| { Some(<$itype>::from_ne_bytes(buf)) })
        }
    };
}

#[macro_export]
macro_rules! read_bytes {
    ($reader: expr, $bytes: expr) => {{
        let mut buf = [0u8; $bytes];
        $reader.read(&mut buf)
            .map(|_| buf)
    }};
}
