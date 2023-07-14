use std::io::{
    Read,
    Seek,
    Cursor,
    SeekFrom,
};
use std::fs::File;

pub trait Stream: Read + Seek {
    fn stream_size(&mut self) -> Result<u64, std::io::Error> {
        let pos = self.stream_position()?;
        let end = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(pos))?;

        Ok(end)
    }

    fn eof(&mut self) -> Result<bool, std::io::Error> {
        let pos = self.stream_position()?;
        let end = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(pos))?;

        Ok(pos >= end)
    }

    fn to_cursor(&mut self) -> Result<Cursor<Vec<u8>>, std::io::Error> {
        let mut vec = Vec::new();
        self.read_to_end(&mut vec)?;

        Ok(Cursor::new(vec))
    }
}
impl Stream for Cursor<Vec<u8>> {
    fn eof(&mut self) -> Result<bool, std::io::Error> {
        let pos = self.stream_position()?;
        
        let size = self.get_ref().len() as u64;
        Ok(pos >= size)
    }

    fn to_cursor(&mut self) -> Result<Cursor<Vec<u8>>, std::io::Error> {
        let this = std::mem::take(self);
        Ok(this)
    }
}
impl Stream for File {}
