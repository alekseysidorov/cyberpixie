use crate::{until_ok, NbResultExt};

struct MaybeBlock {
    attempts: usize,
    value: usize,

    remaining_attemts: usize,
}

impl Default for MaybeBlock {
    fn default() -> Self {
        Self {
            attempts: 5,
            value: 0,

            remaining_attemts: 0,
        }
    }
}

impl MaybeBlock {
    fn poll_me(&mut self) -> nb::Result<usize, ()> {
        if self.remaining_attemts == 0 {
            let value = self.value;

            self.value += 1;
            self.remaining_attemts = self.attempts;
            return Ok(value);
        }

        self.remaining_attemts -= 1;
        Err(nb::Error::WouldBlock)
    }
}

#[test]
fn test_maybe_block() {
    let mut block = MaybeBlock {
        value: 1,
        ..MaybeBlock::default()
    };

    let value = nb::block!(block.poll_me()).unwrap();
    assert_eq!(1, value);
}

#[test]
fn test_filter() {
    let mut block = MaybeBlock::default();

    let value = nb::block!(block.poll_me().filter(|value| *value == 5)).unwrap();
    assert_eq!(5, value);
}

#[test]
fn test_filter_map() {
    let mut block = MaybeBlock::default();

    let value = nb::block!(block.poll_me().filter_map(|value| if value == 5 {
        Some("ready")
    } else {
        None
    }))
    .unwrap();
    assert_eq!("ready", value);
}

#[test]
fn test_until_ok() {
    let mut block = MaybeBlock {
        value: 1,
        ..MaybeBlock::default()
    };

    let poll_me_async = until_ok(|| block.poll_me());
    let value = spin_on::spin_on(poll_me_async).unwrap();

    assert_eq!(value, 1);
}
