use bitflags::bitflags;
use itertools::Itertools;
use nom::bytes::complete::take;
use tap::Pipe;

use crate::error::Error;

bitflags! {
    #[derive(Debug)]
    pub struct StreamMask: u16 {
        const English = 0x00;
    }
}

const QUANTIZATION_TABLE: [i16; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    40, 41, 42, 43, 47, 51, 56, 61, 66, 72, 79, 86, 94, 102, 112, 122, 133,
    145, 158, 173, 189, 206, 225, 245, 267, 292, 318, 348, 379, 414, 452, 493,
    538, 587, 640, 699, 763, 832, 908, 991, 1081, 1180, 1288, 1405, 1534, 1673,
    1826, 1993, 2175, 2373, 2590, 2826, 3084, 3365, 3672, 4008, 4373, 4772,
    5208, 5683, 6202, 6767, 7385, 8059, 8794, 9597, 10472, 11428, 12471, 13609,
    14851, 16206, 17685, 19298, 21060, 22981, 25078, 27367, 29864, 32589,
    -29973, -26728, -23186, -19322, -15105, -10503, -5481, -1, 1, 1, 5481,
    10503, 15105, 19322, 23186, 26728, 29973, -32589, -29864, -27367, -25078,
    -22981, -21060, -19298, -17685, -16206, -14851, -13609, -12471, -11428,
    -10472, -9597, -8794, -8059, -7385, -6767, -6202, -5683, -5208, -4772,
    -4373, -4008, -3672, -3365, -3084, -2826, -2590, -2373, -2175, -1993,
    -1826, -1673, -1534, -1405, -1288, -1180, -1081, -991, -908, -832, -763,
    -699, -640, -587, -538, -493, -452, -414, -379, -348, -318, -292, -267,
    -245, -225, -206, -189, -173, -158, -145, -133, -122, -112, -102, -94, -86,
    -79, -72, -66, -61, -56, -51, -47, -43, -42, -41, -40, -39, -38, -37, -36,
    -35, -34, -33, -32, -31, -30, -29, -28, -27, -26, -25, -24, -23, -22, -21,
    -20, -19, -18, -17, -16, -15, -14, -13, -12, -11, -10, -9, -8, -7, -6, -5,
    -4, -3, -2, -1,
];

#[derive(Debug)]
pub enum Channels {
    Mono,
    Stereo,
}

#[derive(Debug)]
pub enum BitWidth {
    EightBit,
    SixteenBit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelDeltas {
    Mono { delta: i16 },
    Stereo { left_delta: i16, right_delta: i16 },
}

/// Stored as array of samples
#[derive(Debug)]
struct AudioData(pub Vec<i16>);

#[derive(Debug)]
struct DecodingOptions {
    is_first_chunk: bool,
    starting_delta: i16,
    channels: ChannelDeltas,
}

#[derive(Debug)]
struct StereoData {
    bias: Option<(i16, i16)>,
    left_data: Vec<u8>,
    right_data: Vec<u8>,
}

fn separate_channels(
    is_first_chunk: bool,
    data: &[u8],
) -> Result<StereoData, Error> {
    let (data, bias) = if is_first_chunk {
        let (data, left_bias) = crate::take2(data)?;
        let (data, right_bias) = crate::take2(data)?;

        let left_bias = i16::from_le_bytes(left_bias);
        let right_bias = i16::from_le_bytes(right_bias);

        (data, Some((left_bias, right_bias)))
    } else {
        (data, None)
    };

    let length = data.len();

    let mut left_data = Vec::with_capacity(length / 2);
    let mut right_data = Vec::with_capacity(length / 2);

    for (left, right) in data.iter().tuples() {
        left_data.push(*left);
        right_data.push(*right);
    }

    StereoData {
        bias,
        left_data,
        right_data,
    }
    .pipe(Ok)
}

fn from_quantized_values(mut delta: i16, values: &[u8]) -> (Vec<i16>, i16) {
    let mut samples = Vec::with_capacity(values.len());

    for v in values {
        let next_delta = QUANTIZATION_TABLE[*v as usize];
        delta += next_delta as i16;

        samples.push(delta);
    }

    (samples, delta)
}

#[derive(Debug)]
pub enum DecompressedChannelData {
    Mono {
        data: Vec<i16>,
        delta: i16,
    },
    Stereo {
        data: Vec<i16>,
        left_delta: i16,
        right_delta: i16,
        /// has value on the first chunk only
        bias: Option<(i16, i16)>,
    },
}

fn decode_compressed_audio(
    is_first_chunk: bool,
    channel_deltas: &ChannelDeltas,
    data: &[u8],
) -> Result<DecompressedChannelData, Error> {
    match *channel_deltas {
        ChannelDeltas::Mono { delta } => {
            let (data, delta) = from_quantized_values(delta, data);
            DecompressedChannelData::Mono { data, delta }.pipe(Ok)
        }
        ChannelDeltas::Stereo {
            left_delta,
            right_delta,
        } => {
            let stereo_data = separate_channels(is_first_chunk, data)?;
            let (left_data, left_delta) =
                from_quantized_values(left_delta, &stereo_data.left_data);
            let (right_data, right_delta) =
                from_quantized_values(right_delta, &stereo_data.right_data);

            let data: Vec<_> =
                left_data.into_iter().interleave(right_data).collect();

            DecompressedChannelData::Stereo {
                data,
                left_delta,
                right_delta,
                bias: stereo_data.bias,
            }
            .pipe(Ok)
        }
    }
}

#[derive(Debug)]
struct AudioSettings {
    channels: Channels,
    bit_width: BitWidth,
    is_compressed: bool,
    sample_rate: u16,
    min_buffer_length: u16,
}

fn read_init_audio_buffers(data: &[u8]) {
    struct Data {
        unknown: u16,
        flags: u16,
        sample_rate: u16,
        min_buffer_length: u16,
    }
}
