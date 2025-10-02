extern crate ffmpeg_next as ffmpeg;

#[cfg(test)]
mod tests;

use std::{ffi::c_void, io::Write, marker::PhantomData, sync::Once, time::Duration};

static _INIT: Once = Once::new();

#[repr(C)]
#[derive(Debug)]
struct BufferData<'a> {
    ptr: *const u8,
    len: usize,
    pos: usize,

    __marker: PhantomData<&'a [u8]>,
}
impl<'a> BufferData<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            ptr: buffer.as_ptr(),
            len: buffer.len(),
            pos: 0,
            __marker: PhantomData,
        }
    }
}

pub type VideoFrames = Vec<Vec<u8>>;

#[derive(Debug, PartialEq, Clone)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
}

#[derive(PartialEq)]
pub struct Video {
    pub info: VideoInfo,
    pub frames: VideoFrames,
}
impl std::fmt::Debug for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Video")
            .field("info", &self.info)
            .field("frames", &self.frames.len())
            .finish()
    }
}
impl Video {
    pub fn duration(&self) -> std::time::Duration {
        let secs = self.frames.len() as f64 / self.info.frame_rate;
        std::time::Duration::from_secs_f64(secs)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u32,
    pub format: AudioFormat,
}

pub type AudioSamples = Vec<u8>;

#[derive(PartialEq, Eq)]
pub struct Audio {
    pub info: AudioInfo,
    pub samples: AudioSamples,
}
impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Audio")
            .field("samples", &self.samples.len())
            .field("info", &self.info)
            .finish()
    }
}

impl Audio {
    pub fn save_wav<W>(
        &self,
        mut writer: W,
        force_channels: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        W: Write,
    {
        let channels = force_channels.unwrap_or(self.info.channels as u16);

        // WAV header
        let data_size = self.samples.len() as u32;
        let file_size = 36 + data_size;

        writer.write_all(b"RIFF")?;
        writer.write_all(&file_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;

        let channel_width = self.info.format.byte_width() as u32;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?; // fmt chunk size
        writer.write_all(&1u16.to_le_bytes())?; // PCM format
        writer.write_all(&channels.to_le_bytes())?;
        writer.write_all(&self.info.sample_rate.to_le_bytes())?;
        writer
            .write_all(&(self.info.sample_rate * channels as u32 * channel_width).to_le_bytes())?; // byte rate
        writer.write_all(&(channels * channel_width as u16).to_le_bytes())?; // block align
        writer.write_all(&16u16.to_le_bytes())?; // bits per sample

        // data chunk
        writer.write_all(b"data")?;
        writer.write_all(&data_size.to_le_bytes())?;
        writer.write_all(&self.samples)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AudioFormat {
    U8,
    I16,
    I32,
    I64,
    F32,
    F64,
}
impl AudioFormat {
    fn byte_width(&self) -> u8 {
        match self {
            AudioFormat::U8 => 1,
            AudioFormat::I16 => 2,
            AudioFormat::I32 => 4,
            AudioFormat::I64 => 8,
            AudioFormat::F32 => 4,
            AudioFormat::F64 => 8,
        }
    }

    fn from_sample(sample: &ffmpeg::format::Sample) -> Self {
        match sample {
            ffmpeg::format::Sample::None => panic!("no sample format"),
            ffmpeg::format::Sample::U8(_) => AudioFormat::U8,
            ffmpeg::format::Sample::I16(_) => AudioFormat::I16,
            ffmpeg::format::Sample::I32(_) => AudioFormat::I32,
            ffmpeg::format::Sample::I64(_) => AudioFormat::I64,
            ffmpeg::format::Sample::F32(_) => AudioFormat::I32,
            ffmpeg::format::Sample::F64(_) => AudioFormat::I64,
        }
    }
}

unsafe extern "C" fn read_packet(ptr: *mut c_void, buf: *mut u8, buf_size: i32) -> i32 {
    unsafe {
        let buffer_data = &mut *(ptr as *mut BufferData);
        let remaining = buffer_data.len - buffer_data.pos;

        let to_read = remaining.min(buf_size as usize);

        if to_read == 0 {
            return ffmpeg::ffi::AVERROR_EOF;
        }

        std::ptr::copy_nonoverlapping(buffer_data.ptr.add(buffer_data.pos), buf, to_read);

        buffer_data.pos += to_read;
        to_read as i32
    }
}

unsafe extern "C" fn seek(ptr: *mut c_void, offset: i64, seek_mode: i32) -> i64 {
    unsafe {
        let buffer_data = &mut *(ptr as *mut BufferData);

        match seek_mode {
            ffmpeg::ffi::SEEK_SET => {
                if offset >= 0 && offset <= buffer_data.len as i64 {
                    buffer_data.pos = offset as usize;
                    offset
                } else {
                    -1
                }
            }

            ffmpeg::ffi::SEEK_CUR => {
                let new_pos = buffer_data.pos as i64 + offset;
                if new_pos >= 0 && new_pos <= buffer_data.len as i64 {
                    buffer_data.pos = new_pos as usize;
                    new_pos
                } else {
                    -1
                }
            }

            ffmpeg::ffi::SEEK_END => {
                let new_pos = buffer_data.len as i64 + offset;
                if new_pos >= 0 && new_pos <= buffer_data.len as i64 {
                    buffer_data.pos = new_pos as usize;
                    new_pos
                } else {
                    -1
                }
            }

            ffmpeg::ffi::AVSEEK_SIZE => buffer_data.len as i64,
            _ => -1,
        }
    }
}

struct AudioStream<'a>(ffmpeg::Stream<'a>);
impl<'a> AudioStream<'a> {
    fn from_input(input: &'a ffmpeg::format::context::Input) -> Option<Self> {
        let stream = input.streams().best(ffmpeg::media::Type::Audio)?;

        Some(Self(stream))
    }
}

struct VideoStream<'a>(ffmpeg::Stream<'a>);
impl<'a> VideoStream<'a> {
    fn from_input(input: &'a ffmpeg::format::context::Input) -> Option<Self> {
        let stream = input.streams().best(ffmpeg::media::Type::Video)?;
        Some(Self(stream))
    }

    fn frame_rate(&self) -> f64 {
        rational_to_double(self.0.rate())
    }
}

struct AudioContext {
    decoder: ffmpeg::decoder::Audio,
    frame: ffmpeg::frame::Audio,
}
impl AudioContext {
    fn new(stream: &AudioStream) -> Result<Self, ffmpeg::Error> {
        let context = ffmpeg::codec::Context::from_parameters(stream.0.parameters())?;
        let decoder = context.decoder().audio()?;

        Ok(Self {
            decoder,
            frame: ffmpeg::frame::Audio::empty(),
        })
    }

    fn decode_frame(&mut self) -> Vec<u8> {
        self.frame.data(0).to_vec()
    }

    /// **NOTE**: Remember to also get the end frames
    fn receive_packet_frames<'a>(
        &'a mut self,
        packet: &ffmpeg::Packet,
    ) -> Result<impl Iterator<Item = Vec<u8>> + use<'a>, ffmpeg::Error> {
        self.decoder.send_packet(packet)?;

        Ok(std::iter::from_fn(|| {
            self.decoder
                .receive_frame(&mut self.frame)
                .map(|()| self.decode_frame())
                .ok()
        }))
    }

    fn receive_end_frames<'a>(
        &'a mut self,
    ) -> Result<impl Iterator<Item = Vec<u8>> + use<'a>, ffmpeg::Error> {
        self.decoder.send_eof()?;

        Ok(std::iter::from_fn(|| {
            self.decoder
                .receive_frame(&mut self.frame)
                .map(|()| self.decode_frame())
                .ok()
        }))
    }
}

struct VideoContext {
    decoder: ffmpeg::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
    frame: ffmpeg::frame::Video,
    rgb_frame: ffmpeg::frame::Video,
}
impl VideoContext {
    fn new(stream: &VideoStream) -> Result<Self, ffmpeg::Error> {
        let codec_params = stream.0.parameters();

        let context = ffmpeg::codec::Context::from_parameters(codec_params)?;
        let decoder = context.decoder().video()?;
        let scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;

        Ok(Self {
            decoder,
            scaler,
            frame: ffmpeg::frame::Video::empty(),
            rgb_frame: ffmpeg::frame::Video::empty(),
        })
    }

    fn decode_frame(&mut self) -> Result<Vec<u8>, ffmpeg::Error> {
        self.scaler.run(&self.frame, &mut self.rgb_frame)?;
        Ok(extract_frame_data(&self.rgb_frame))
    }

    /// **NOTE**: Remember to also get the end frames
    fn receive_packet_frames<'a>(
        &'a mut self,
        packet: &ffmpeg::Packet,
    ) -> Result<impl Iterator<Item = Vec<u8>> + use<'a>, ffmpeg::Error> {
        self.decoder.send_packet(packet)?;

        Ok(std::iter::from_fn(|| {
            self.decoder
                .receive_frame(&mut self.frame)
                .and_then(|()| self.decode_frame())
                .ok()
        }))
    }

    fn receive_end_frames<'a>(
        &'a mut self,
    ) -> Result<impl Iterator<Item = Vec<u8>> + use<'a>, ffmpeg::Error> {
        self.decoder.send_eof()?;

        Ok(std::iter::from_fn(|| {
            self.decoder
                .receive_frame(&mut self.frame)
                .and_then(|()| self.decode_frame())
                .ok()
        }))
    }
}

#[derive(Debug)]
pub struct VideoChannel {
    pub info: VideoInfo,
    pub channel: std::sync::mpsc::Receiver<Vec<u8>>,
}

#[derive(Debug)]
pub struct AudioChannel {
    pub info: AudioInfo,
    pub channel: std::sync::mpsc::Receiver<Vec<u8>>,
}

#[derive(Debug)]
pub struct AVChannel {
    pub audio: AudioChannel,
    pub video: VideoChannel,
}

#[derive(Debug)]
pub struct State;

#[allow(clippy::new_without_default)]
impl State {
    pub fn new() -> Self {
        _INIT.call_once(|| ffmpeg::init().expect("Failed to init FFmpeg"));

        Self
    }

    pub fn mve_channel(&self, data: &[u8]) -> Result<AVChannel, ffmpeg::Error> {
        let mut input = Input::from_buffer(BufferData::new(data))?;

        let (video_stream, video_stream_index) = {
            let stream = VideoStream::from_input(&input.input).expect("Couldn't find video stream");
            let index = stream.0.index();
            (stream, index)
        };

        let (audio_stream, audio_stream_index) = {
            let stream = AudioStream::from_input(&input.input).expect("Couldn't find audio stream");
            let index = stream.0.index();

            (stream, index)
        };

        let mut video_context = VideoContext::new(&video_stream)?;
        let mut audio_context = AudioContext::new(&audio_stream)?;

        let frame_rate = video_stream.frame_rate();

        let video_info = VideoInfo {
            frame_rate,
            width: video_context.decoder.width(),
            height: video_context.decoder.height(),
        };

        let audio_info = AudioInfo {
            format: AudioFormat::from_sample(&audio_context.decoder.format()),
            sample_rate: audio_context.decoder.rate(),
            channels: audio_context.decoder.channels() as u32,
        };

        let (video_tx, video_rx) = std::sync::mpsc::channel();
        let (audio_tx, audio_rx) = std::sync::mpsc::channel();

        let send_video_frame = |frame| {
            video_tx.send(frame).expect("Failed to send video frame");
        };

        let send_audio_frame = |frame| {
            audio_tx.send(frame).expect("Failed to send audio frame");
        };

        for (stream, packet) in input.input.packets() {
            match stream.index() {
                i if i == video_stream_index => {
                    video_context
                        .receive_packet_frames(&packet)?
                        .for_each(send_video_frame);
                }
                i if i == audio_stream_index => {
                    audio_context
                        .receive_packet_frames(&packet)?
                        .for_each(send_audio_frame);
                }
                _ => {}
            }
        }

        video_context
            .receive_end_frames()?
            .for_each(send_video_frame);

        audio_context
            .receive_end_frames()?
            .for_each(send_audio_frame);

        Ok(AVChannel {
            video: VideoChannel {
                info: video_info,
                channel: video_rx,
            },
            audio: AudioChannel {
                info: audio_info,
                channel: audio_rx,
            },
        })
    }

    pub fn mve(&self, data: &[u8]) -> Result<(Video, Audio), ffmpeg::Error> {
        let mut input = Input::from_buffer(BufferData::new(data))?;

        let (video_stream, video_stream_index) = {
            let stream = VideoStream::from_input(&input.input).expect("Couldn't find video stream");
            let index = stream.0.index();
            (stream, index)
        };

        let (audio_stream, audio_stream_index) = {
            let stream = AudioStream::from_input(&input.input).expect("Couldn't find audio stream");
            let index = stream.0.index();

            (stream, index)
        };

        let mut video_context = VideoContext::new(&video_stream)?;
        let mut audio_context = AudioContext::new(&audio_stream)?;

        let frame_rate = video_stream.frame_rate();

        let mut frames = vec![];
        let mut samples = vec![];

        for (stream, packet) in input.input.packets() {
            match stream.index() {
                i if i == video_stream_index => {
                    video_context
                        .receive_packet_frames(&packet)?
                        .for_each(|frame| {
                            frames.push(frame);
                        });
                }
                i if i == audio_stream_index => {
                    audio_context
                        .receive_packet_frames(&packet)?
                        .for_each(|frame| samples.extend_from_slice(&frame));
                }
                _ => {}
            }
        }

        video_context
            .receive_end_frames()?
            .for_each(|frame| frames.push(frame));

        audio_context
            .receive_end_frames()?
            .for_each(|frame| samples.extend_from_slice(&frame));

        let video = Video {
            info: VideoInfo {
                frame_rate,
                width: video_context.decoder.width(),
                height: video_context.decoder.height(),
            },
            frames,
        };

        let audio = Audio {
            info: AudioInfo {
                format: AudioFormat::from_sample(&audio_context.decoder.format()),
                sample_rate: audio_context.decoder.rate(),
                channels: audio_context.decoder.channels() as u32,
            },
            samples,
        };

        Ok((video, audio))
    }

    pub fn acm_channel(&self, data: &[u8]) -> Result<AudioChannel, ffmpeg::Error> {
        let mut input = Input::from_buffer(BufferData::new(data))?;

        let (audio_stream, audio_stream_index) = {
            let stream = AudioStream::from_input(&input.input).expect("Couldn't find audio stream");
            let index = stream.0.index();

            (stream, index)
        };

        let mut audio_context = AudioContext::new(&audio_stream)?;

        let (audio_tx, audio_rx) = std::sync::mpsc::channel();

        let audio_info = AudioInfo {
            format: AudioFormat::from_sample(&audio_context.decoder.format()),
            sample_rate: audio_context.decoder.rate(),
            channels: audio_context.decoder.channels() as u32,
        };

        let send_audio_frame = |frame| {
            audio_tx.send(frame).expect("Failed to send audio frame");
        };

        for (stream, packet) in input.input.packets() {
            match stream.index() {
                i if i == audio_stream_index => {
                    audio_context
                        .receive_packet_frames(&packet)?
                        .for_each(send_audio_frame);
                }
                _ => {}
            }
        }

        audio_context
            .receive_end_frames()?
            .for_each(send_audio_frame);

        let audio = AudioChannel {
            info: audio_info,
            channel: audio_rx,
        };

        Ok(audio)
    }

    pub fn acm(&self, data: &[u8]) -> Result<Audio, ffmpeg::Error> {
        let mut input = Input::from_buffer(BufferData::new(data))?;

        let (audio_stream, audio_stream_index) = {
            let stream = AudioStream::from_input(&input.input).expect("Couldn't find audio stream");
            let index = stream.0.index();

            (stream, index)
        };

        let mut audio_context = AudioContext::new(&audio_stream)?;

        let mut samples = vec![];

        for (stream, packet) in input.input.packets() {
            match stream.index() {
                i if i == audio_stream_index => {
                    audio_context
                        .receive_packet_frames(&packet)?
                        .for_each(|frame| samples.extend_from_slice(&frame));
                }
                _ => {}
            }
        }

        audio_context
            .receive_end_frames()?
            .for_each(|frame| samples.extend_from_slice(&frame));

        let audio = Audio {
            samples,
            info: AudioInfo {
                format: AudioFormat::from_sample(&audio_context.decoder.format()),
                sample_rate: audio_context.decoder.rate(),
                channels: audio_context.decoder.channels() as u32,
            },
        };

        Ok(audio)
    }
}

fn rational_to_double(x: ffmpeg::Rational) -> f64 {
    x.numerator() as f64 / x.denominator() as f64
}

struct Input<'a> {
    input: ffmpeg::format::context::Input,
    __lifetime: std::marker::PhantomData<&'a [u8]>,
}
impl<'a> Input<'a> {
    #[allow(unused)]
    fn duration(&self) -> Option<Duration> {
        let duration = self.input.duration();
        if duration < 0 {
            None
        } else {
            let secs = self.input.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64;
            Some(Duration::from_secs_f64(secs))
        }
    }

    fn from_buffer(mut buffer_data: BufferData<'a>) -> Result<Self, ffmpeg::Error> {
        let input = unsafe {
            let avio_buffer_size = 4096;
            let avio_buffer = ffmpeg::ffi::av_malloc(avio_buffer_size) as *mut u8;

            let avio_ctx = ffmpeg::ffi::avio_alloc_context(
                avio_buffer,
                avio_buffer_size as i32,
                0,
                &mut buffer_data as *mut _ as *mut c_void,
                Some(read_packet),
                None,
                Some(seek),
            );

            let mut format_ctx = ffmpeg::ffi::avformat_alloc_context();
            (*format_ctx).pb = avio_ctx;

            match ffmpeg::ffi::avformat_open_input(
                &mut format_ctx,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
            ) {
                0 => match ffmpeg::ffi::avformat_find_stream_info(format_ctx, std::ptr::null_mut())
                {
                    r if r >= 0 => Ok(ffmpeg::format::context::Input::wrap(format_ctx)),
                    e => {
                        ffmpeg::ffi::avformat_close_input(&mut format_ctx);
                        Err(ffmpeg::Error::from(e))
                    }
                },
                e => Err(ffmpeg::Error::from(e)),
            }
        }?;

        let this = Self {
            input,
            __lifetime: std::marker::PhantomData,
        };
        Ok(this)
    }
}

fn extract_frame_data(frame: &ffmpeg::util::frame::video::Video) -> Vec<u8> {
    let mut data = Vec::new();
    let bytes_per_pixel = 3; // RGB24
    let width = frame.width() as usize;
    let height = frame.height() as usize;

    let plane_data = frame.data(0);
    let linesize = frame.stride(0);

    for y in 0..height {
        let row_start = y * linesize;
        let row_end = row_start + (width * bytes_per_pixel);
        data.extend_from_slice(&plane_data[row_start..row_end]);
    }

    data
}
