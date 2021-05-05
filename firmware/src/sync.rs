use core::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};

use riscv::interrupt::{self, CriticalSection, Mutex};

pub struct RwLock<T>(Mutex<RefCell<T>>);

impl<T> RwLock<T> {
    pub const fn new(inner: T) -> Self {
        Self(Mutex::new(RefCell::new(inner)))
    }

    pub fn read<F, R>(&self, read: F) -> Result<R, BorrowError>
    where
        F: FnOnce(Ref<T>) -> R,
    {
        interrupt::free(|cs| {
            let cell = self.0.borrow(cs);
            let inner = cell.try_borrow()?;

            let result = read(inner);
            Ok(result)
        })
    }

    pub fn write<F, R>(&self, write: F) -> Result<R, BorrowMutError>
    where
        F: FnOnce(RefMut<T>) -> R,
    {
        interrupt::free(|cs| {
            let cell = self.0.borrow(cs);
            let inner = cell.try_borrow_mut()?;

            let result = write(inner);
            Ok(result)
        })
    }

    pub fn inner<'cs>(&'cs self, cs: &'cs CriticalSection) -> &'cs RefCell<T> {
        self.0.borrow(cs)
    }
}
