#[derive(Debug, Default, Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self{
            red: r,
            green: g,
            blue: b,
        }
    }
}

impl From<&[u8]> for Color {
    fn from(slice: &[u8]) -> Self {
        Self{
            red: slice[0],
            green: slice[1],
            blue: slice[2],
        }
    }
}

impl From<[u8; 3]> for Color {
    fn from(slice: [u8; 3]) -> Self {
        Self{
            red: slice[0],
            green: slice[1],
            blue: slice[2],
        }
    }
}
