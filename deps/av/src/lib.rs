extern crate ffmpeg_next as ffmpeg;

#[cfg(test)]
mod tests;

use std::{
    ffi::c_void,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    sync::{Arc, Mutex, Once},
};

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

pub struct Video {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<Vec<u8>>,
    pub frame_rate: f64,
}
impl std::fmt::Debug for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Video")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("frames", &self.frames.len())
            .field("frame_rate", &self.frame_rate)
            .finish()
    }
}
impl Video {
    pub fn duration(&self) -> std::time::Duration {
        let secs = self.frames.len() as f64 / self.frame_rate;
        std::time::Duration::from_secs_f64(secs)
    }
}

#[derive(Debug)]
pub struct Audio {
    pub samples: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: AudioFormat,
}

impl Audio {
    pub fn save_wav<W>(&self, mut writer: W) -> Result<(), Box<dyn std::error::Error>>
    where
        W: Write,
    {
        // WAV header
        let data_size = self.samples.len() as u32;
        let file_size = 36 + data_size;

        writer.write_all(b"RIFF")?;
        writer.write_all(&file_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;

        // fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?; // fmt chunk size
        writer.write_all(&1u16.to_le_bytes())?; // PCM format
        writer.write_all(&(self.channels as u16).to_le_bytes())?;
        writer.write_all(&self.sample_rate.to_le_bytes())?;
        writer.write_all(&(self.sample_rate * self.channels * 2).to_le_bytes())?; // byte rate
        writer.write_all(&((self.channels * 2) as u16).to_le_bytes())?; // block align
        writer.write_all(&16u16.to_le_bytes())?; // bits per sample

        // data chunk
        writer.write_all(b"data")?;
        writer.write_all(&data_size.to_le_bytes())?;
        writer.write_all(&self.samples)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum AudioFormat {
    /// 16-bit signed integer
    I16,
    /// 32-bit float
    F32,
    /// 32-bit signed integer
    I32,
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

pub fn decode_audio_buffer(buffer: &[u8]) -> Result<Audio, ffmpeg::Error> {
    let mut buffer_data = BufferData::new(buffer);

    unsafe {
        // Create AVIO context (same as video example)
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

        let ret = ffmpeg::ffi::avformat_open_input(
            &mut format_ctx,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
        );

        if ret < 0 {
            return Err(ffmpeg::Error::from(ret));
        }

        let ret = ffmpeg::ffi::avformat_find_stream_info(format_ctx, std::ptr::null_mut());
        if ret < 0 {
            return Err(ffmpeg::Error::from(ret));
        }

        // Find audio stream
        let mut audio_stream_index = -1;
        for i in 0..(*format_ctx).nb_streams {
            let stream = *(*format_ctx).streams.add(i as usize);
            if (*(*stream).codecpar).codec_type == ffmpeg::ffi::AVMediaType::AVMEDIA_TYPE_AUDIO {
                audio_stream_index = i as i32;
                break;
            }
        }

        if audio_stream_index == -1 {
            return Err(ffmpeg::Error::StreamNotFound);
        }

        let stream = *(*format_ctx).streams.add(audio_stream_index as usize);
        let codecpar = (*stream).codecpar;

        // Find audio decoder
        let codec = ffmpeg::ffi::avcodec_find_decoder((*codecpar).codec_id);
        if codec.is_null() {
            return Err(ffmpeg::Error::DecoderNotFound);
        }

        let codec_ctx = ffmpeg::ffi::avcodec_alloc_context3(codec);
        let ret = ffmpeg::ffi::avcodec_parameters_to_context(codec_ctx, codecpar);
        if ret < 0 {
            return Err(ffmpeg::Error::from(ret));
        }

        let ret = ffmpeg::ffi::avcodec_open2(codec_ctx, codec, std::ptr::null_mut());
        if ret < 0 {
            return Err(ffmpeg::Error::from(ret));
        }

        // Create resampler context
        let swr_ctx = ffmpeg::ffi::swr_alloc();
        ffmpeg::ffi::av_opt_set_int(
            swr_ctx as *mut c_void,
            c"in_channel_layout".as_ptr(),
            match (*codecpar).ch_layout.nb_channels {
                1 => ffmpeg::ffi::AV_CH_LAYOUT_MONO as i64,
                2 => ffmpeg::ffi::AV_CH_LAYOUT_STEREO as i64,
                x => panic!("Unexpected number of channels: {x}"),
            },
            0,
        );
        ffmpeg::ffi::av_opt_set_int(
            swr_ctx as *mut c_void,
            c"out_channel_layout".as_ptr(),
            ffmpeg::ffi::AV_CH_LAYOUT_STEREO as i64,
            0,
        );
        ffmpeg::ffi::av_opt_set_int(
            swr_ctx as *mut c_void,
            c"in_sample_rate".as_ptr(),
            (*codec_ctx).sample_rate as i64,
            0,
        );
        ffmpeg::ffi::av_opt_set_int(
            swr_ctx as *mut c_void,
            c"out_sample_rate".as_ptr(),
            44100,
            0,
        );
        ffmpeg::ffi::av_opt_set_sample_fmt(
            swr_ctx as *mut c_void,
            c"in_sample_fmt".as_ptr(),
            (*codec_ctx).sample_fmt,
            0,
        );
        ffmpeg::ffi::av_opt_set_sample_fmt(
            swr_ctx as *mut c_void,
            c"out_sample_fmt".as_ptr(),
            ffmpeg::ffi::AVSampleFormat::AV_SAMPLE_FMT_S16,
            0,
        );

        let ret = ffmpeg::ffi::swr_init(swr_ctx);
        if ret < 0 {
            return Err(ffmpeg::Error::from(ret));
        }

        let mut audio_data = Vec::new();
        let frame = ffmpeg::ffi::av_frame_alloc();
        let packet = ffmpeg::ffi::av_packet_alloc();

        // Allocate output buffer
        let max_out_samples = 4096;
        let out_buffer_size = max_out_samples * 2 * 2; // 2 channels * 2 bytes per sample
        let out_buffer = ffmpeg::ffi::av_malloc(out_buffer_size) as *mut i16;

        while ffmpeg::ffi::av_read_frame(format_ctx, packet) >= 0 {
            if (*packet).stream_index == audio_stream_index {
                let ret = ffmpeg::ffi::avcodec_send_packet(codec_ctx, packet);
                if ret < 0 {
                    continue;
                }

                while ffmpeg::ffi::avcodec_receive_frame(codec_ctx, frame) >= 0 {
                    // Resample
                    let out_samples = ffmpeg::ffi::swr_convert(
                        swr_ctx,
                        &mut (out_buffer as *mut u8) as *mut *mut u8,
                        max_out_samples as i32,
                        (*frame).data.as_ptr() as *const *const u8,
                        (*frame).nb_samples,
                    );

                    if out_samples > 0 {
                        let samples_bytes = out_samples as usize * 2 * 2; // 2 channels * 2 bytes
                        let sample_slice =
                            std::slice::from_raw_parts(out_buffer as *const u8, samples_bytes);
                        audio_data.extend_from_slice(sample_slice);
                    }
                }
            }
            ffmpeg::ffi::av_packet_unref(packet);
        }

        // Cleanup
        ffmpeg::ffi::av_frame_free(&mut (frame as *mut _));
        ffmpeg::ffi::av_packet_free(&mut (packet as *mut _));
        ffmpeg::ffi::avcodec_free_context(&mut (codec_ctx as *mut _));
        ffmpeg::ffi::swr_free(&mut (swr_ctx as *mut _));
        ffmpeg::ffi::avformat_close_input(&mut (format_ctx as *mut _));
        ffmpeg::ffi::av_free(out_buffer as *mut c_void);

        Ok(Audio {
            samples: audio_data,
            sample_rate: 44100,
            channels: 2,
            format: AudioFormat::I16,
        })
    }
}

#[derive(Debug)]
pub struct State;

#[allow(clippy::new_without_default)]
impl State {
    pub fn new() -> Self {
        _INIT.call_once(|| ffmpeg::init().expect("Failed to init FFmpeg"));

        Self
    }

    pub fn mve(&self, data: &[u8]) -> Result<Video, ffmpeg::Error> {
        let mut buffer_data = BufferData::new(data);

        unsafe {
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
                x if x < 0 => return Err(ffmpeg::Error::from(x)),
                _ => {}
            };

            match ffmpeg::ffi::avformat_find_stream_info(format_ctx, std::ptr::null_mut()) {
                x if x < 0 => return Err(ffmpeg::Error::from(x)),
                _ => {}
            }

            let video_stream_index = (0..(*format_ctx).nb_streams)
                .find(|x| {
                    let stream = (*(*format_ctx).streams).add(*x as usize);
                    (*(*stream).codecpar).codec_type == ffmpeg::ffi::AVMediaType::AVMEDIA_TYPE_VIDEO
                })
                .ok_or(ffmpeg::Error::StreamNotFound)?;

            let stream = (*(*format_ctx).streams).add(video_stream_index as usize);
            let codecpar = (*stream).codecpar;
            let fps = ffmpeg::ffi::av_q2d((*stream).r_frame_rate);

            let codec = ffmpeg::ffi::avcodec_find_decoder((*codecpar).codec_id);
            if codec.is_null() {
                return Err(ffmpeg::Error::DecoderNotFound);
            }

            let codec_ctx = ffmpeg::ffi::avcodec_alloc_context3(codec);
            match ffmpeg::ffi::avcodec_parameters_to_context(codec_ctx, codecpar) {
                x if x < 0 => return Err(ffmpeg::Error::from(x)),
                _ => {}
            }

            match ffmpeg::ffi::avcodec_open2(codec_ctx, codec, std::ptr::null_mut()) {
                x if x < 0 => return Err(ffmpeg::Error::from(x)),
                _ => {}
            }

            let sws_ctx = ffmpeg::ffi::sws_getContext(
                (*codec_ctx).width,
                (*codec_ctx).height,
                (*codec_ctx).pix_fmt,
                (*codec_ctx).width,
                (*codec_ctx).height,
                ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_RGB24,
                ffmpeg::ffi::SWS_BILINEAR,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            let mut frames = vec![];
            let frame = ffmpeg::ffi::av_frame_alloc();
            let rgb_frame = ffmpeg::ffi::av_frame_alloc();

            let num_bytes = ffmpeg::ffi::av_image_get_buffer_size(
                ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_RGB24,
                (*codec_ctx).width,
                (*codec_ctx).height,
                1,
            );

            let buffer = ffmpeg::ffi::av_malloc(num_bytes as usize) as *mut u8;
            ffmpeg::ffi::av_image_fill_arrays(
                (*rgb_frame).data.as_mut_ptr(),
                (*rgb_frame).linesize.as_mut_ptr(),
                buffer,
                ffmpeg::ffi::AVPixelFormat::AV_PIX_FMT_RGB24,
                (*codec_ctx).width,
                (*codec_ctx).height,
                1,
            );
            let packet = ffmpeg::ffi::av_packet_alloc();

            while ffmpeg::ffi::av_read_frame(format_ctx, packet) >= 0 {
                if (*packet).stream_index == video_stream_index as i32 {
                    match ffmpeg::ffi::avcodec_send_packet(codec_ctx, packet) {
                        x if x < 0 => return Err(ffmpeg::Error::from(x)),
                        _ => {}
                    }

                    while ffmpeg::ffi::avcodec_receive_frame(codec_ctx, frame) >= 0 {
                        ffmpeg::ffi::sws_scale(
                            sws_ctx,
                            (*frame).data.as_ptr() as *const *const u8,
                            (*frame).linesize.as_ptr(),
                            0,
                            (*codec_ctx).height,
                            (*rgb_frame).data.as_ptr(),
                            (*rgb_frame).linesize.as_ptr(),
                        );

                        let frame_data = extract_rgb_frame_data(
                            rgb_frame,
                            (*codec_ctx).width,
                            (*codec_ctx).height,
                        );
                        frames.push(frame_data);
                    }
                }
                ffmpeg::ffi::av_packet_unref(packet);
            }

            let width = (*codec_ctx).width;
            let height = (*codec_ctx).height;

            // Cleanup
            ffmpeg::ffi::av_frame_free(&mut (frame as *mut _));
            ffmpeg::ffi::av_frame_free(&mut (rgb_frame as *mut _));
            ffmpeg::ffi::av_packet_free(&mut (packet as *mut _));
            ffmpeg::ffi::avcodec_free_context(&mut (codec_ctx as *mut _));
            ffmpeg::ffi::sws_freeContext(sws_ctx);
            ffmpeg::ffi::avformat_close_input(&mut (format_ctx as *mut _));
            ffmpeg::ffi::av_free(buffer as *mut c_void);

            Ok(Video {
                width: width as u32,
                height: height as u32,
                frames,
                frame_rate: fps,
            })
        }
    }

    pub fn acm(&self, data: &[u8]) {
        todo!()
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

unsafe fn extract_rgb_frame_data(
    frame: *const ffmpeg::ffi::AVFrame,
    width: i32,
    height: i32,
) -> Vec<u8> {
    unsafe {
        let mut data = Vec::new();
        let bytes_per_pixel = 3; // RGB24
        let linesize = (*frame).linesize[0] as usize;
        let frame_data = (*frame).data[0];

        for y in 0..height as usize {
            let row_start = y * linesize;
            let _row_end = row_start + (width as usize * bytes_per_pixel);
            let row_slice = std::slice::from_raw_parts(
                frame_data.add(row_start),
                width as usize * bytes_per_pixel,
            );
            data.extend_from_slice(row_slice);
        }

        data
    }
}
