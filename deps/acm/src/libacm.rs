use crate::{ 
    Acm,
    SampleType,
    error::AcmError,
};

use libc::{
    c_uint,
    c_int,
    c_void,
};
use std::io::{ 
    Cursor,
    Read,
    Seek,
    SeekFrom,
};
use std::mem::{
    MaybeUninit,
    size_of,
};

mod sys {
    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

const ENDIANNESS: i32 =
    if cfg!(target_endian = "big") { 1 }
    else { 0 };

unsafe extern "C" fn read_func(ptr: *mut c_void, size: c_int, n: c_int, datasrc: *mut c_void) -> c_int {
    let data: *mut Cursor<Vec<u8>> = datasrc.cast();
    let total_size = n * size;

    let mut buffer = vec![0u8; total_size as usize];
    let buf_ptr = buffer.as_mut_ptr().cast();

    if let Ok(res) = (*data).read(&mut buffer) {
        ptr.copy_from(buf_ptr, total_size as usize);
        res as c_int / size
    } else {
        -1
    }
}
unsafe extern "C" fn seek_func(datasrc: *mut c_void, offset: c_int, whence: c_int) -> c_int {
    let data: *mut Cursor<Vec<u8>> = datasrc.cast();

    let seek_from = match whence {
        libc::SEEK_SET => SeekFrom::Start(offset as u64),
        libc::SEEK_CUR => SeekFrom::Current(offset as i64),
        libc::SEEK_END => SeekFrom::End(offset as i64),
        _ => return -1,
    };

    if let Ok(_) = (*data).seek(seek_from) { 0 }
    else { -1 }
}
unsafe extern "C" fn close_func(_datasrc: *mut c_void) -> c_int {
    0
}
unsafe extern "C" fn get_length_func(datasrc: *mut c_void) -> c_int {
    let data: *mut Cursor<Vec<u8>> = datasrc.cast();
    (*data).get_ref().len() as c_int
}


pub fn read_data(mut data: Cursor<Vec<u8>>, force_channels: Option<i32>) -> Result<Acm, AcmError> {
    let channels;
    let sample_rate;
    let total_values;
    let mut samples;
    let word_len = size_of::<SampleType>();

    let callbacks = sys::acm_io_callbacks {
        read_func: Some(read_func),
        seek_func: Some(seek_func),
        close_func: Some(close_func),
        get_length_func: Some(get_length_func),
    };

    unsafe {
        let acm;

        let mut acm_tmp = MaybeUninit::<*mut sys::ACMStream>::uninit();
        let acm_ptr = acm_tmp.as_mut_ptr();
        let data_ptr: *mut Cursor<Vec<u8>> = &mut data;
        let res = sys::acm_open_decoder(acm_ptr, data_ptr.cast(), callbacks, force_channels.unwrap_or(0));
        if res < 0 {
            return Err(AcmError::StreamError);
        }
        acm = acm_tmp.assume_init();

        channels = sys::acm_channels(acm);
        sample_rate = sys::acm_rate(acm);
        total_values = (*acm).total_values;

        samples = Vec::<SampleType>::with_capacity(total_values as usize);

        let mut bytes_read = 0;
        let output_size = total_values as usize;
        let total_bytes = output_size * word_len;

        let mut buffer = [0u8; 4096];

        while bytes_read < total_bytes {
            let buf_ptr = buffer.as_mut_ptr().cast();
            let buf_len = buffer.len() as c_uint;
            let res = sys::acm_read_loop(
                acm,
                buf_ptr,
                buf_len,
                ENDIANNESS,
                word_len as c_int,
                1
            );
            if res == 0 { break; }
            else if res > 0 { 
                bytes_read += res as usize; 
                let b2: [SampleType; 2048] = std::mem::transmute(buffer);
                
                let mut insert_count = 0;
                for s in b2 {
                    if insert_count == res / word_len as i32 {
                        break;
                    }
                    samples.push(s);
                    insert_count += 1;
                }
            } else { return Err(AcmError::StreamError); }
        }
        sys::acm_close(acm);
    };

    //pad with zero if missing samples
    while samples.len() < total_values as usize {
        samples.push(0);
    }

    Ok(Acm {
        channels,
        sample_rate,
        samples,
    })
}

