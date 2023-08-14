use std::io::Read;

pub trait FromBytes<const N: usize> {
    fn from_bytes_ne(bytes: [u8; N]) -> Self;
    fn from_bytes_be(bytes: [u8; N]) -> Self;
    fn from_bytes_le(bytes: [u8; N]) -> Self;
}

macro_rules! impl_from_bytes {
    ($t:ty) => {
        impl_from_bytes!($t, { std::mem::size_of::<$t>() });
    };
    ($t:ty, $sz:expr) => {
        impl FromBytes<$sz> for $t {
            fn from_bytes_ne(bytes: [u8; $sz]) -> Self {
                <$t>::from_ne_bytes(bytes)
            }

            fn from_bytes_be(bytes: [u8; $sz]) -> Self {
                <$t>::from_be_bytes(bytes)
            }

            fn from_bytes_le(bytes: [u8; $sz]) -> Self {
                <$t>::from_le_bytes(bytes)
            }
        }
    };
}

impl_from_bytes!(i8);
impl_from_bytes!(i16);
impl_from_bytes!(i32);
impl_from_bytes!(i64);

impl_from_bytes!(u8);
impl_from_bytes!(u16);
impl_from_bytes!(u32);
impl_from_bytes!(u64);

impl_from_bytes!(f32);
impl_from_bytes!(f64);

impl FromBytes<1> for bool {
    fn from_bytes_ne(bytes: [u8; 1]) -> Self {
        bytes[0] != 0
    }
    fn from_bytes_be(bytes: [u8; 1]) -> Self {
        Self::from_bytes_ne(bytes)
    }
    fn from_bytes_le(bytes: [u8; 1]) -> Self {
        Self::from_bytes_ne(bytes)
    }
}

pub enum ReadMode {
    BE,
    LE,
    NE,
}

pub fn read_type<R, const N: usize>(read_mode: ReadMode, data: &mut impl Read) -> std::io::Result<R> where R: FromBytes<N> {
    let mut buf = [0u8; N];

    let _ = data.read_exact(&mut buf)?;
    match read_mode {
        ReadMode::BE => Ok(R::from_bytes_be(buf)),
        ReadMode::LE => Ok(R::from_bytes_ne(buf)),
        ReadMode::NE => Ok(R::from_bytes_le(buf)),
    }
}

pub fn read_bytes<const N: usize>(data: &mut impl Read) -> std::io::Result<[u8; N]> {
    let mut buf = [0u8; N];
    data.read_exact(&mut buf)?;

    Ok(buf)
}

