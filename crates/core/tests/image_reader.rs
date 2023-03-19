use cyberpixie_core::{proto::types::Hertz, service::Image, storage::ImageReader, ExactSizeRead};
use embedded_io::{
    blocking::{Read, Seek},
    SeekFrom,
};

const BLOCK_SIZE: usize = 32;

fn make_block_device(s: impl AsRef<[u8]>) -> Vec<u8> {
    let mut bytes = s.as_ref().to_owned();

    let add_bytes = BLOCK_SIZE - bytes.len() % BLOCK_SIZE;
    bytes.extend(std::iter::repeat(0).take(add_bytes));
    bytes
}

#[test]
fn test_make_block_device() {
    let block = make_block_device("hello world");
    assert_eq!(block.len(), 32);

    // More big block
    let block = make_block_device("Section 1.10.32 of 'de Finibus Bonorum");
    assert_eq!(block.len(), 64);

    // The biggest one
    let block = make_block_device(
        "Section 1.10.32 of 'de Finibus Bonorum et Malorum', written by Cicero in 45 BC",
    );
    assert_eq!(block.len(), 96);
}

#[test]
fn test_image_reader_read_exact_lesser_than_block() {
    let s = "hello world";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    let mut buf = vec![0_u8; image_len];
    reader.bytes.read_exact(&mut buf).unwrap();

    assert_eq!(s, String::from_utf8_lossy(&buf));
    // Go to the image beginning and try to read again
    reader.bytes.seek(SeekFrom::Start(0)).unwrap();
    reader.bytes.read_exact(&mut buf).unwrap();
    assert_eq!(s, String::from_utf8_lossy(&buf));
}

#[test]
fn test_image_reader_read_parts_lesser_than_block() {
    let s = "hello world";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    let mut out = vec![];
    loop {
        let mut buf = [0_u8; 3];
        let bytes_read = reader.bytes.read(&mut buf).unwrap();
        if bytes_read == 0 {
            break;
        }

        out.extend_from_slice(&buf[0..bytes_read]);
    }

    assert_eq!(s, String::from_utf8_lossy(&out));
}

#[test]
fn test_image_reader_read_exact_multiple_blocks() {
    let s = "The standard Lorem Ipsum passage, used since the 1500s \
        Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
        tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, ";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    let mut buf = vec![0_u8; image_len];
    reader.bytes.read_exact(&mut buf).unwrap();

    assert_eq!(s, String::from_utf8_lossy(&buf));
    // Check that there are no bytes in the reader
    assert_eq!(0, reader.bytes.read(&mut buf).unwrap());
    // Go to the image beginning and try to read again
    reader.bytes.seek(SeekFrom::Start(0)).unwrap();
    reader.bytes.read_exact(&mut buf).unwrap();
    assert_eq!(s, String::from_utf8_lossy(&buf));
}

#[test]
fn test_image_reader_read_parts_multiple_blocks() {
    let s = "The standard Lorem Ipsum passage, used since the 1500s \
        Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
        tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, ";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    fn read_image(reader: &mut Image<impl ExactSizeRead + Seek>) -> Vec<u8> {
        let mut out = vec![];
        loop {
            let mut buf = [0_u8; 3];
            let bytes_read = reader.bytes.read(&mut buf).unwrap();
            if bytes_read == 0 {
                break;
            }

            out.extend_from_slice(&buf[0..bytes_read]);
        }
        out
    }

    let out = read_image(&mut reader);
    assert_eq!(s, String::from_utf8_lossy(&out));
    // Go to the image beginning and try to read again
    reader.bytes.seek(SeekFrom::Start(0)).unwrap();
    let out = read_image(&mut reader);
    assert_eq!(s, String::from_utf8_lossy(&out));
}

#[test]
fn test_image_reader_read_big_parts_multiple_blocks() {
    let s = "The standard Lorem Ipsum passage, used since the 1500s \
        Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
        tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, ";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    let mut out = vec![];
    loop {
        let mut buf = [0_u8; BLOCK_SIZE + 4];
        let bytes_read = reader.bytes.read(&mut buf).unwrap();
        if bytes_read == 0 {
            break;
        }

        out.extend_from_slice(&buf[0..bytes_read]);
    }

    assert_eq!(s, String::from_utf8_lossy(&out));
}

#[test]
fn test_image_reader_read_single_byte_buf_multiple_blocks() {
    let s = "The standard Lorem Ipsum passage, used since the 1500s \
        Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
        tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, ";

    let blocks = make_block_device(s);

    let image_len = s.len();
    let mut reader = Image {
        refresh_rate: Hertz(50),
        bytes: ImageReader::<_, BLOCK_SIZE>::new(blocks.as_ref(), image_len),
    };

    let mut out = vec![];
    loop {
        let mut buf = [0_u8; 1];
        let bytes_read = reader.bytes.read(&mut buf).unwrap();
        if bytes_read == 0 {
            break;
        }

        out.extend_from_slice(&buf[0..bytes_read]);
    }

    assert_eq!(s, String::from_utf8_lossy(&out));
}
