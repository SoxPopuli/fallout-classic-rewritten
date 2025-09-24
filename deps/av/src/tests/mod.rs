use std::path::Path;

use super::*;

fn read_n<T, const N: usize>(mut reader: T) -> std::io::Result<[u8; N]>
where
    T: Read,
{
    let mut output = [0u8; N];

    reader.read(&mut output).map(|_| output)
}

#[test]
fn x2() {
    let input = include_bytes!("../../../../data/iplogo.mve");
    let mut buffer_data = BufferData::new(input);

    let video = unsafe {
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

        let mut input = match ffmpeg::ffi::avformat_open_input(
            &mut format_ctx,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
        ) {
            0 => match ffmpeg::ffi::avformat_find_stream_info(format_ctx, std::ptr::null_mut()) {
                r if r >= 0 => Ok(ffmpeg::format::context::Input::wrap(format_ctx)),
                e => {
                    ffmpeg::ffi::avformat_close_input(&mut format_ctx);
                    Err(ffmpeg::Error::from(e))
                }
            },
            e => Err(ffmpeg::Error::from(e)),
        }
        .unwrap();

        let video_stream = input.streams().best(ffmpeg::media::Type::Video).unwrap();

        fn rational_to_double(x: ffmpeg::Rational) -> f64 {
            x.numerator() as f64 / x.denominator() as f64
        }

        let video_stream_index = video_stream.index();
        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(video_stream.parameters()).unwrap();

        let mut decoder = context_decoder.decoder().video().unwrap();
        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .unwrap();

        let mut frames = vec![];
        let mut frame = ffmpeg::util::frame::video::Video::empty();
        let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();

        let frame_rate = rational_to_double(video_stream.rate());

        for (stream, packet) in input.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet).unwrap();

                while decoder.receive_frame(&mut frame).is_ok() {
                    scaler.run(&frame, &mut rgb_frame).unwrap();
                    let frame_data = extract_frame_data(&rgb_frame);
                    frames.push(frame_data);
                }
            }
        }

        decoder.send_eof().unwrap();
        while decoder.receive_frame(&mut frame).is_ok() {
            scaler.run(&frame, &mut rgb_frame).unwrap();
            let frame_data = extract_frame_data(&rgb_frame);
            frames.push(frame_data);
        }

        let width = decoder.width();
        let height = decoder.height();

        Video {
            frames,
            frame_rate,
            width,
            height,
        }
    };

    panic!("{video:?}");
}

#[test]
fn x() {
    let input = include_bytes!("../../../../data/iplogo.mve");
    // let state = State::new();
    // state.acm(input);
    // let video = state.mve(input).unwrap();
    // println!("{}", video.width);
    // println!("{}", video.height);
    // println!("{}", video.frame_rate);

    let out_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("iplogo.wav");

    let output = std::fs::File::create(out_path).unwrap();

    let audio = decode_audio_buffer(input).unwrap();

    audio.save_wav(output).unwrap();

    /*
    let out_path = Path::new( env!("CARGO_MANIFEST_DIR") )
        .join("iplogo.png");

    // panic!("{}", out_path.display());
    let output = std::fs::File::create(out_path).unwrap();

    let mut encoder = png::Encoder::new(output, video.width, video.height);

    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(video.frames.len() as u32, 0).unwrap();

    let mut writer = encoder.write_header().unwrap();

    for f in video.frames {
        writer.write_image_data(&f).unwrap();
    }
    */
}
