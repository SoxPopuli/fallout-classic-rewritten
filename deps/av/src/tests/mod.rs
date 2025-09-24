use super::*;

fn read_n<T, const N: usize>(mut reader: T) -> std::io::Result<[u8; N]>
    where T: Read
{
    let mut output = [0u8; N];

    reader.read(&mut output)
       .map(|_| output)
}

#[test]
fn seek_tests() {
    let buf = [0, 1, 2, 3, 4, 5];
    let mut stream = Stream::new(buf.to_vec());

    stream.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(read_n::<_, 3>(&mut stream).unwrap(), [0, 1, 2]);
    assert_eq!(stream.pointer, 3);

    assert_eq!(read_n::<_, 3>(&mut stream).unwrap(), [3, 4, 5]);
    assert_eq!(stream.pointer, 6);

    stream.seek(SeekFrom::Start(2));
    assert_eq!(read_n::<_, 2>(&mut stream).unwrap(), [2, 3]);
    assert_eq!(stream.pointer, 4);

    stream.seek(SeekFrom::Current(-2));
    assert_eq!(read_n::<_, 2>(&mut stream).unwrap(), [2, 3]);
    assert_eq!(stream.pointer, 4);

    stream.seek(SeekFrom::End(-2));
    assert_eq!(read_n::<_, 2>(&mut stream).unwrap(), [4, 5]);
    assert_eq!(stream.pointer, 6);
}
