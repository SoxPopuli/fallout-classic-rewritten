#[cfg(test)]
mod tests;

pub mod tree;
use tree::{ FileTree, Node, FileEntry, FileState, };
mod error;
use error::{ DatError, DatError::* };

use std::{
    path::Path, 
    fs::File, 
    io::{Read, Seek, Cursor, SeekFrom, stdout, Write},
    error::Error, collections::HashMap,
    cell::{RefCell, RefMut},
};
use common::{ Stream, read_num };

#[derive(PartialEq, Eq, Debug, Default, Clone, Copy)]
pub enum Version {
    #[default]
    None,

    Dat1, 
    Dat2,
}

macro_rules! read_dat {
    ($file:expr, $t: ty) => {{
        read_num!($file, $t, be).ok_or(DatError::ReadError)
    }};
}


pub struct DatFile {
    file: RefCell<Box<dyn Stream>>,
    version: Version,
    pub registry: FileTree,
}

impl DatFile {
    pub fn get_version(&self) -> &Version {
        &self.version
    }


    pub fn open(mut stream: impl Stream + 'static) -> Result<DatFile, Box<dyn Error>> {
        let mut buf = [0u8; 4];

        let mut read_be_int = || -> Result<i32, Box<dyn Error>> {
            let r = stream.read(&mut buf)?;
            if r < 4 {
                return Err("too few bytes read".into());
            }

            Ok(i32::from_be_bytes(buf))
        };

        let dir_count = read_be_int()?;
        let id = read_be_int()?;
        let zero = read_be_int()?;

        let version;
        if dir_count > 0 &&
            (id == 0x0A || id == 0x5E) &&
                zero == 0 {
            version = Version::Dat1;
        } else {
            version = Version::Dat2;
        }

        let mut dat = Self{
            file: RefCell::new(Box::new(stream)),
            version,
            registry: FileTree::new(),
        };
        match version {
            Version::Dat1 => { dat.read_dat1(dir_count)?; },
            Version::Dat2 => { dat.read_dat2()?; },
            _ => { }
        }

        Ok(dat)
    }

    fn read_dat1(&mut self, dir_count: i32) -> Result<(), Box<dyn Error>> {
        let _sum = self.read_int32_be()?;

        //directory names
        let mut dir_names = Vec::with_capacity(dir_count as usize);
        for _ in 0..dir_count {
            let len = self.read_byte_be()?;
            let mut dir_name_bytes = vec![0u8; len as usize];
            self.file.get_mut().read(&mut dir_name_bytes)?;

            let name = String::from_utf8(dir_name_bytes)?;
            dir_names.push(name);
        }

        //directory content
        for i in 0..dir_count {
            let file_count = self.read_int32_be()?;
            let _unknown1 = self.read_int32_be()?;
            let _unknown2 = self.read_int32_be()?;
            let _unknown3 = self.read_int32_be()?;

            for _ in 0..file_count {
                let file_name_len = self.read_byte_be()?;
                let mut file_name_bytes = vec![0u8; file_name_len as usize];
                self.file.get_mut().read(&mut file_name_bytes)?;

                let file_name = String::from_utf8(file_name_bytes)?;
                let file_attributes = self.read_int32_be()?;
                let file_offset = self.read_int32_be()?;
                let file_size = self.read_int32_be()?;
                let file_size_compressed = self.read_int32_be()?;

                let state = (file_size_compressed == 0 || file_attributes == 0x20)
                    .then_some(FileState::Uncompressed)
                    .unwrap_or(FileState::Compressed { size: file_size_compressed as usize });

                let entry = FileEntry{
                    offset: file_offset as usize,
                    size: file_size as usize,
                    state,
                };

                let full_name;
                if dir_names[i as usize] == "." {
                    full_name = file_name;
                } else {
                    full_name = format!("{}\\{}", dir_names[i as usize], file_name);
                }

                let name = full_name.to_ascii_lowercase();
                self.registry.insert_unsorted(&name, entry)?;

                //self.registry.entry(full_name.to_ascii_lowercase()).or_insert(entry);
            }
        }

        self.registry.sort().expect("failed to sort tree, possible mutex issue");

        Ok(())
    }

    fn read_dat2(&mut self) -> Result<(), Box<dyn Error>> {
        let file = self.file.get_mut();
        file.seek(SeekFrom::End(-8))?;
        
        let tree_size = read_num!(file, u32, le).ok_or("failed to read tree size")?;
        let data_size = read_num!(file, u32, le).ok_or("failed to read data size")?;

        let dir_tree_start = data_size - tree_size - 4;
        file.seek(SeekFrom::Start(dir_tree_start as u64 - 4))?;

        let file_count = read_num!(file, u32, le).ok_or("failed to read file count")?;

        //load the entire tree into a buffer :)
        let mut dir_tree_buffer = Cursor::new(vec![0u8; tree_size as usize]);
        file.read(dir_tree_buffer.get_mut())?;

        let mut entries_read = 0;
        dir_tree_buffer.seek(SeekFrom::Start(0))?;
        while entries_read < file_count {
            if dir_tree_buffer.position() >= dir_tree_buffer.get_ref().len() as u64 {
                break;
            }
            let name_size = read_num!(dir_tree_buffer, u32, le).ok_or("failed to read from file")?;
            
            let mut str_buf = vec![0u8; name_size as usize];
            dir_tree_buffer.read(&mut str_buf)?;

            let file_type = read_num!(dir_tree_buffer, u8, le).ok_or("failed to read from file")?;
            let real_size = read_num!(dir_tree_buffer, u32, le).ok_or("failed to read from file")?;
            let packed_size = read_num!(dir_tree_buffer, u32, le).ok_or("failed to read from file")?;
            let offset = read_num!(dir_tree_buffer, u32, le).ok_or("failed to read from file")?;

            let state = match file_type {
                0 => FileState::Uncompressed,
                _ => FileState::Compressed { size: packed_size as usize }
            };

            let entry = FileEntry {
                offset: offset as usize,
                size: real_size as usize,
                state,
            };

            let name = String::from_utf8(str_buf)?;
            let name = name.to_ascii_lowercase();

            self.registry.insert_unsorted(&name, entry)?;
            //self.registry.entry(name.to_ascii_lowercase()).or_insert(entry);

            entries_read += 1;
        }

        self.registry.sort().expect("failed to sort tree, possible mutex issue");

        Ok(())
    }

    fn read_int32_be(&mut self) -> Result<i32, Box<dyn Error>> { read_int32_be(&mut self.file.get_mut()) }
    fn read_int32_le(&mut self) -> Result<i32, Box<dyn Error>> { read_int32_le(&mut self.file.get_mut()) }
    fn read_byte_be(&mut self) -> Result<i8, Box<dyn Error>> { read_byte_be(&mut self.file.get_mut()) }
    fn read_byte_le(&mut self) -> Result<i8, Box<dyn Error>> { read_byte_le(&mut self.file.get_mut()) }

    pub fn unpack_file(&self, entry: &FileEntry) -> Result<Vec<u8>, Box<dyn Error>> {
        self.file.borrow_mut().seek(SeekFrom::Start(entry.offset as u64))?;

        match self.version {
            Version::Dat1 => self.unpack_dat1(entry),
            Version::Dat2 => self.unpack_dat2(entry),
            Version::None => Err("invalid dat version".into())
        }
    }

    pub fn unpack_to_cursor(&self, entry: &FileEntry) -> Result<Cursor<Vec<u8>>, Box<dyn Error>> {
        let file = self.unpack_file(entry)?;
        Ok(Cursor::new(file))
    }

    fn unpack_dat1(&self, entry: &FileEntry) -> Result<Vec<u8>, Box<dyn Error>> {
        let output = match entry.state {
            FileState::Uncompressed => {
                let mut buffer = vec![0u8; entry.size];
                self.file.borrow_mut().read(&mut buffer)?;

                buffer
            },
            FileState::Compressed { size: _ } => {
                self.decompress_lzss(entry)?

                //let mut input = self.get_entry_data(entry)?;
                //let output_size = entry.size;
                //lzss::decompress_lzss(&mut input, output_size)
            }
        };

        Ok(output)
    }

    fn unpack_dat2(&self, entry: &FileEntry) -> Result<Vec<u8>, Box<dyn Error>> {
        let output = match entry.state {
            FileState::Uncompressed => {
                let mut buffer = vec![0u8; entry.size];
                self.file.borrow_mut().read(&mut buffer)?;

                buffer
            },
            FileState::Compressed { size } => {
                self.decompress_zip(entry, size).ok_or("zip decompression error")?
            }
        };

        Ok(output)
    }

    pub fn get_entry_data(&self, entry: &FileEntry) -> Result<Vec<u8>, DatError> {
        self.file
            .borrow_mut()
            .seek(SeekFrom::Start(entry.offset as u64))
            .map_err(|_| ReadError)?;
        let mut buf;

        match entry.state {
            FileState::Uncompressed => {
                buf = vec![0u8; entry.size];
            },
            FileState::Compressed { size } => {
                buf = vec![0u8; size];
            }
        };

        self.file
            .borrow_mut()
            .read(&mut buf)
            .map_err(|_| ReadError)?;

        Ok(buf)
    }

    fn decompress_lzss(&self, entry: &FileEntry) -> Result<Vec<u8>, DatError> {
        let input = self.get_entry_data(entry)?;
        let input = Cursor::new(input);

        Self::decompress_lzss_inner(input, entry.size)
    }

    fn decompress_lzss_inner(mut input: Cursor<Vec<u8>>, output_size: usize) -> Result<Vec<u8>, DatError> {
        let mut output = Cursor::new( vec![0u8; output_size] );

        let mut dictionary = [0u8; 4096];
        let mut dict_offset;
        let mut dict_index;
        let mut n;
        let mut f;
        let mut l;

        let input_end = input.get_ref().len() as u64;

        while input.position() < input_end {
            n = read_dat!(input, i16)?;

            if n == 0 {
                return Err(LZSSError)
            } else if n < 0 {
                let mut buf = vec![0u8; (-n) as usize];
                input.read(&mut buf).map_err(|_| LZSSError)?;
                output.write_all(&buf).map_err(|_| LZSSError)?;
            } else {
                dict_offset = dictionary.len() - 18;
                dictionary.fill(' ' as u8);

                let block_end = input.position() + n as u64;
                while input.position() < block_end {
                    f = read_dat!(input, u8)?;
                    let mut i = 0;
                    while i < 8 && input.position() < block_end {

                        if (f & 1) != 0 {
                            let byte = read_dat!(input, u8)?;
                            output.write(&[byte]).map_err(|_| LZSSError)?;
                            dictionary[dict_offset] = byte;
                            dict_offset += 1;
                            if dict_offset >= dictionary.len() {
                                dict_offset = 0;
                            }
                        } else {
                            dict_index = read_dat!(input, u8)? as usize;
                            l = read_dat!(input, u8)?;
                            let l_high = ((l & 0xF0) as usize) << 4;
                            dict_index |= l_high;
                            l &= 0x0F;

                            for _ in 0..l+3 {
                                let byte = dictionary[dict_index];
                                output.write(&[byte]).map_err(|_| LZSSError)?;
                                dictionary[dict_offset] = byte;

                                dict_index += 1;
                                dict_offset += 1;

                                if dict_index >= dictionary.len() {
                                    dict_index = 0;
                                }
                                if dict_offset >= dictionary.len() {
                                    dict_offset = 0;
                                }
                            }
                        }

                        i += 1;
                        f >>= 1;
                    }
                }
            }

        }

        Ok(output.into_inner())

    }

    fn decompress_zip(&self, entry: &FileEntry, compressed_size: usize) -> Option<Vec<u8>> {
        let mut file = self.file.borrow_mut();

        //let _sig = read_num!(file, u16, le)?;

        let mut input_buffer = vec![0u8; compressed_size - 0];
        let mut output_buffer = vec![0u8; entry.size];

        file.read(&mut input_buffer).ok()?;

        let mut decoder = flate2::bufread::ZlibDecoder::new(input_buffer.as_slice());
        decoder.read(&mut output_buffer).ok()?;

        Some(output_buffer)
    }
}

    

fn read_int32(file: &mut Box<dyn Stream>, read_func: fn([u8; 4]) -> i32) -> Result<i32, Box<dyn Error>> {
    let mut buf = [0u8; 4];
    let sz = file.read(&mut buf)?;
    if sz < 4 { return Err("not enough bytes read".into()); }

    return Ok(read_func(buf));
}
fn read_int32_be(file: &mut Box<dyn Stream>) -> Result<i32, Box<dyn Error>> { read_int32(file, i32::from_be_bytes) }
fn read_int32_le(file: &mut Box<dyn Stream>) -> Result<i32, Box<dyn Error>> { read_int32(file, i32::from_le_bytes) }

fn read_byte(file: &mut Box<dyn Stream>, read_func: fn([u8; 1]) -> i8) -> Result<i8, Box<dyn Error>> {
    let mut buf = [0u8; 1];
    let sz = file.read(&mut buf)?;
    if sz < 1 { return Err("not enough bytes read".into()); }

    return Ok(read_func(buf));
}
fn read_byte_be(file: &mut Box<dyn Stream>) -> Result<i8, Box<dyn Error>> { read_byte(file, i8::from_be_bytes) }
fn read_byte_le(file: &mut Box<dyn Stream>) -> Result<i8, Box<dyn Error>> { read_byte(file, i8::from_le_bytes) }
