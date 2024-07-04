use crate::{take_n, utils::MapPair as _};
use nom::{bytes::complete::take, IResult};
use std::fmt::Debug;

#[derive(Debug)]
pub enum OpcodeType {
    EndOfStream,
    EndOfChunk,
    CreateTimer,
    InitializeAudioBuffers,
    StartStopAudio,
    InitializeVideoBuffers,
    SendBufferToDisplay,
    AudioFrame,
    InitializeVideoMode,
    CreateGradient,
    SetPalette,
    SetPaletteEntriesCompressed,
    SetDecodingMap,
    VideoData,
}
impl OpcodeType {
    pub fn try_from_int(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::EndOfStream),
            0x01 => Some(Self::EndOfChunk),
            0x02 => Some(Self::CreateTimer),
            0x03 => Some(Self::InitializeAudioBuffers),
            0x04 => Some(Self::StartStopAudio),
            0x05 => Some(Self::InitializeVideoBuffers),
            0x07 => Some(Self::SendBufferToDisplay),
            0x08 => Some(Self::AudioFrame),
            0x09 => Some(Self::AudioFrame),
            0x0a => Some(Self::InitializeVideoMode),
            0x0b => Some(Self::CreateGradient),
            0x0c => Some(Self::SetPalette),
            0x0d => Some(Self::SetPaletteEntriesCompressed),
            0x0f => Some(Self::SetDecodingMap),
            0x11 => Some(Self::VideoData),

            _ => None,
        }
    }
}

pub struct Opcode<'a> {
    pub length: u32,
    pub typ: Option<OpcodeType>,
    pub version: u8,
    pub data: &'a [u8],
}
impl<'a> Opcode<'a> {
    pub fn parse(data: &'a [u8]) -> IResult<&[u8], Self> {
        let (data, length) = take_n(data)?.map_second(u16::from_le_bytes);
        let (data, typ) = take_n(data)?.map_second(u8::from_le_bytes);
        let (data, version) = take_n(data)?.map_second(u8::from_le_bytes);
        let (rest, data) = take(length as usize)(data)?;

        Ok((
            rest,
            Self {
                length: length as u32,
                typ: OpcodeType::try_from_int(typ),
                version,
                data,
            },
        ))
    }
}
impl<'a> Debug for Opcode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opcode")
            .field("type", &self.typ)
            .field("version", &self.version)
            .field("length", &self.length)
            .finish()
    }
}
