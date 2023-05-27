use cyberpixie_core::{
    proto::types::Hertz,
    storage::{ImageReader, Image, ImageLines},
    ExactSizeRead,
};
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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
    reader.rewind().unwrap();
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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
    let mut reader = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(blocks.as_ref(), image_len),
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

#[test]
fn test_image_lines_cycle_nyan_cat() {
    let image = image::load_from_memory(include_bytes!("../../../assets/nyan_cat_48.png"))
        .unwrap()
        .to_rgb8();

    // Convert image to raw bytes
    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.extend(rgb.0);
    }
    // dbg!(raw.len());
    // std::fs::write("nyan_cat_48.raw", &raw).unwrap();

    let image = Image::<ImageReader<_, _, BLOCK_SIZE>> {
        refresh_rate: Hertz(50),
        bytes: ImageReader::new_in_array(raw.as_ref(), raw.len()),
    };

    let mut lines: ImageLines<ImageReader<&[u8], _, BLOCK_SIZE>, _> =
        ImageLines::new(image, 48, vec![0_u8; 512]);
    let first_line: Vec<_> = lines.next_line().unwrap().collect();
    // Render a lot of lines
    for _ in 1..360 {
        let _line = lines.next_line().unwrap();
    }
    // After several cycles we should return back to the first image line
    let line: Vec<_> = lines.next_line().unwrap().collect();
    assert_eq!(first_line, line);
    // Check that the next line is not equal the first one
    let line: Vec<_> = lines.next_line().unwrap().collect();
    assert_ne!(first_line, line);
}
