use std::convert::Infallible;

use cyberpixie_core::storage::BlockReader;

const BLOCK_SIZE: usize = 32;

#[test]
fn test_block_reader_smoke() {
    let blocks_count = 10;

    let blocks = (0..blocks_count)
        .flat_map(|block| (0..BLOCK_SIZE).map(move |_| block as u8))
        .collect::<Vec<u8>>();

    let block_reader = &blocks.as_slice() as &dyn BlockReader<BLOCK_SIZE, Error = Infallible>;

    let mut block_buf = [0_u8; BLOCK_SIZE];
    for i in 0..blocks_count {
        block_reader.read_block(i, &mut block_buf).unwrap();

        assert_eq!(block_buf, [i as u8; BLOCK_SIZE]);
    }
}

#[test]
fn test_block_reader_partial_read() {
    let blocks = [117_u8; BLOCK_SIZE].as_slice();

    let block_reader = &blocks as &dyn BlockReader<BLOCK_SIZE, Error = Infallible>;

    let mut read_buf = [0_u8; 4];
    block_reader.read_block(0, &mut read_buf).unwrap();

    assert_eq!(read_buf, [117_u8; 4]);
}
