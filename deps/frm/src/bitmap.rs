#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self{ 
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bitmap {
    pub width: u64,
    pub height: u64,

    pub pixels: Vec<Color>,
}

