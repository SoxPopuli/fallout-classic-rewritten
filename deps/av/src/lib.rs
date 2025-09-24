extern crate ez_ffmpeg as ffmpeg;

#[cfg(test)]
mod tests;

use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex, Once},
};

static _INIT: Once = Once::new();

mod seek_mode {
    /// Seek to an absolute position.
    pub const SEEK_SET: i32 = 0;
    /// Seek relative to the current position.
    pub const SEEK_CUR: i32 = 1;
    /// Seek relative to the end of the stream.
    pub const SEEK_END: i32 = 2;
    /// Find the next hole in a sparse file (Linux only).
    pub const SEEK_HOLE: i32 = 3;
    /// Find the next data block in a sparse file (Linux only).
    pub const SEEK_DATA: i32 = 4;
    /// Seek using byte offset instead of timestamps.
    pub const AVSEEK_FLAG_BYTE: i32 = 2;
    /// Query the total size of the stream.
    pub const AVSEEK_SIZE: i32 = 65536;
    /// Force seeking, even if normally restricted.
    pub const AVSEEK_FORCE: i32 = 131072;
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
struct Stream {
    buf: Vec<u8>,
    pointer: usize,
}
impl Stream {
    pub fn new(buf: Vec<u8>) -> Self {
        Self { buf, pointer: 0 }
    }

    pub fn to_input(self) -> ffmpeg::Input {
        let data = Arc::new(Mutex::new(self));

        let data_read = data.clone();
        let data_seek = data.clone();

        ffmpeg::Input::new_by_read_callback(move |mut buf| {
            let data = data_read.clone();
            let mut lock = data.lock().unwrap();

            lock.read(&mut buf).map(|x| x as i32).unwrap()
        })
        .set_seek_callback(move |pos, mode| {
            let data = data_seek.clone();
            let mut lock = data.lock().unwrap();
            match mode {
                seek_mode::SEEK_CUR => lock.seek(SeekFrom::Current(pos)).unwrap() as i64,
                seek_mode::SEEK_END => lock.seek(SeekFrom::End(pos)).unwrap() as i64,
                seek_mode::SEEK_SET => lock.seek(SeekFrom::Start(pos as u64)).unwrap() as i64,
                seek_mode::AVSEEK_SIZE => lock.buf.len() as i64,

                other => panic!("unsupported seek mode: {other}"),
            }
        })
    }
}
impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match (&self.buf[self.pointer..]).read(buf) {
            Ok(x) => {
                self.pointer += x;
                Ok(x)
            }
            Err(e) => Err(e),
        }
    }
}
impl Seek for Stream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(x) => {
                if (x as usize) < self.buf.len() {
                    self.pointer = x as usize;
                    Ok(self.pointer as u64)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("Offset {x} is out of bounds {}", self.buf.len()),
                    ))
                }
            }
            SeekFrom::End(x) => {
                let len = self.buf.len() as i64;
                if x <= 0 && x > -len {
                    self.pointer = (len - 1 - x) as usize;
                    Ok(self.pointer as u64)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("Offset {x} is out of bounds {}", self.buf.len()),
                    ))
                }
            }
            SeekFrom::Current(x) => {
                let ptr = (self.pointer as i64) + x;
                self.pointer = ptr as usize;

                if self.pointer >= 0 && self.pointer < self.buf.len() {
                    Ok(self.pointer as u64)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("Offset {x} is out of bounds {}", self.buf.len()),
                    ))
                }
            }
        }
    }
}

pub struct State;
impl State {
    pub fn new() -> Self {
        _INIT.call_once(|| {});

        Self
    }

    pub fn acm(&self, data: &[u8]) {
        // let ctx = ffmpeg::FfmpegContext::builder()
        //     .input(input)
        //     .output(output)
        //     .build()
        //     .unwrap();

        // let scheduler = ctx.start().unwrap();
        // scheduler.wait().unwrap();
    }
}
