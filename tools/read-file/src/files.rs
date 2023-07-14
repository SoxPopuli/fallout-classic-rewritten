use std::{
    fs::File,
    io::{
        Write,
        stdout, 
        Read,
        Cursor,
    },
    mem::{ size_of, size_of_val },
};

pub enum FileType {
    Pal,
    Frm,
    Dat,
    Acm,
    Mve,
}

impl FileType {
    pub fn new(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pal" => Some(FileType::Pal),
            "frm" => Some(FileType::Frm),
            "dat" => Some(FileType::Dat),
            "acm" => Some(FileType::Acm),
            "mve" => Some(FileType::Mve),
            _ => None
        }
    }

    pub fn open(&self, file: File, palette: Option<String>) {
        match self {
            Self::Pal => open_pal(file),
            Self::Frm => open_frm(file, palette.expect("frm file requires palette")),
            Self::Dat => (),
            Self::Acm => open_acm(file),
            Self::Mve => open_mve(file),
        }
    }

    pub fn inspect(&self, file: File) {
        match self {
            Self::Pal => inspect_pal(file),
            Self::Frm => inspect_frm(file),
            Self::Dat => (),
            Self::Acm => (),
            Self::Mve => inspect_mve(file),
        }
    }
}

#[repr(C)]
struct WavHeader {
    riff: [u8; 4],
    size: u32,
    wave: [u8; 4],
    fmt: [u8; 4],
    wave_size: u32,
    wave_type: u16,
    channels: u16,
    sample_rate: u32,
    bytes_per_sec: u32,
    alignment: u16,
    bits_per_sample: u16,
    data_header: [u8; 4],
    data_size: u32,
}

impl WavHeader {
    pub fn new(acm: &acm::Acm) -> Self {
        let word_len = size_of_val(&acm.samples[0]);
        let data_size = acm.samples.len() * word_len;
        let bps = acm.sample_rate as u32 * acm.channels as u32 * word_len as u32;
        let bits = word_len * 8;
        let alignment = bits * acm.channels as usize * 8;

        WavHeader {
            riff: ['R', 'I', 'F', 'F'].map(|x| x as u8),
            size: (size_of::<WavHeader>() + data_size - 8) as u32,
            wave: ['W', 'A', 'V', 'E'].map(|x| x as u8),
            fmt:  ['f', 'm', 't', ' '].map(|x| x as u8),
            wave_size: 16,
            wave_type: 0x01,
            channels: acm.channels as u16,
            sample_rate: acm.sample_rate,
            bytes_per_sec: bps,
            alignment: alignment as u16,
            bits_per_sample: bits as u16,
            data_header: ['d', 'a', 't', 'a'].map(|x| x as u8),
            data_size: data_size as u32,
        }
    }
}

fn open_mve(file: File) {

}

fn inspect_mve(file: File) {
    let mve = mve::MveFile::open(file).unwrap();
}

fn open_acm(file: File) {
    let acm = acm::Acm::open(file, Some(1)).unwrap();
    let wav = acm.write_to_wav().unwrap();
    let mut output = stdout().lock();
    output.write_all(&wav).unwrap();
}

fn inspect_pal(mut file: File) {
    let pal = pal::PalFile::open(&mut file).unwrap();

    println!("Pal file:");
    for c in pal.colors.iter() {
        println!("R: {}, G: {}, B: {}", c.red, c.green, c.blue);
    }
}

fn inspect_frm(mut file: File) {
    let frm = frm::FrmFile::open(&mut file).unwrap();

    println!("fps:                  {:?}", frm.fps);
    println!("action_frame:         {:?}", frm.action_frame);
    println!("frames per direction: {:?}", frm.frames_per_direction);
    println!("shifts:               {:?}", frm.shifts);
    println!("offsets:              {:?}", frm.frame_offsets);
    println!("frames:               {:?}", frm.frames.len());
}

fn open_pal(mut file: File) {
    let pal = pal::PalFile::open(&mut file).unwrap();

    println!("<head>");
    println!("<style>");
    println!(".inner {{ color: white; background: rgba(0, 0, 0, 0.6); }}");
    println!("</style>");
    println!("</head>");


    println!("<body style=\"background-color: black\">");

    for (i, c) in pal.colors.iter().enumerate() {
        let style_string = format!("style=\"\
            background-color: rgb({}, {}, {})
        \"", c.red, c.green, c.blue);

        let text = format!("{:<3}: {}, {}, {}", i, c.red, c.green, c.blue);
        let text_elem = format!("<span class=\"inner\">{}</span>", text);

        println!("<p {}>{}</p>", style_string, text_elem);
    }

    println!("</body>");
}

fn open_frm(mut file: File, palette: String) {
    let pal = pal::PalFile::open(
        &mut File::open(palette).unwrap()
    ).unwrap();

    let frm = frm::FrmFile::open(&mut file).unwrap();

    let bitmap = frm.decode(&pal);
    let ref first = bitmap[0];

    let mut pixels = vec![0u8; first.pixels.len() * 4];
    for (i, p) in first.pixels.iter().enumerate() {
        let offset = i * 4;

        pixels[offset + 0] = p.red;
        pixels[offset + 1] = p.green;
        pixels[offset + 2] = p.blue;
        pixels[offset + 3] = p.alpha;
    }

    let png = lodepng::encode32(&pixels, first.width as usize, first.height as usize).unwrap();
    stdout().write(&png).unwrap();
}
