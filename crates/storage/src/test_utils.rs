//! Test helpers

use core::convert::Infallible;

use embedded_storage::{ReadStorage, Storage};

/// In-memory embedded-storage backend.
pub struct MemoryBackend(Vec<u8>);

/// Since tests have a short lifetime we can allow memory to leak without any side effects.
#[must_use]
pub fn leaked_buf() -> &'static mut [u8] {
    Box::leak(vec![0; 512].into_boxed_slice())
}

impl Default for MemoryBackend {
    fn default() -> Self {
        // Allocate 4MB memory by default, it would be enough for any possible
        // kind of tests
        Self(vec![0_u8; 4 * 1024 * 1024])
    }
}

impl ReadStorage for MemoryBackend {
    type Error = Infallible;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let from = offset as usize;
        let to = from + bytes.len();
        bytes.copy_from_slice(&self.0[from..to]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.0.len()
    }
}

impl Storage for MemoryBackend {
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let from = offset as usize;
        let to = from + bytes.len();
        self.0[from..to].copy_from_slice(bytes);
        dbg!("write", offset, &self.0[512..516]);
        Ok(())
    }
}

#[test]
fn test_embedded_storage_in_memory() {
    let mut backend = MemoryBackend::default();

    let expected_data = b"some bytes string".as_slice();

    for offset in 0..1024 {
        backend.write(offset, expected_data).unwrap();

        let mut actual_data = vec![0_u8; expected_data.len()];
        assert_ne!(expected_data, actual_data);
        backend.read(offset, &mut actual_data).unwrap();
        assert_eq!(expected_data, actual_data);
    }
}
