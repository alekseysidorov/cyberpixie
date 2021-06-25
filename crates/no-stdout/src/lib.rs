#![cfg_attr(not(feature = "std"), no_std)]

use core::{
    fmt::{self},
    hint,
    sync::atomic::{AtomicUsize, Ordering},
};

static STATE: AtomicUsize = AtomicUsize::new(UNINITIALIZED);
static mut STDOUT: &dyn StdOut = &NopOut;

pub trait StdOut: Send + 'static {
    fn write_bytes(&self, bytes: &[u8]) -> fmt::Result;
    fn write_str(&self, s: &str) -> fmt::Result;
    fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result;
    fn flush(&self) -> fmt::Result;
}

// Three different states can occur during the program lifecycle:
//
// The stdout is uninitialized yet.
const UNINITIALIZED: usize = 0;
// The stdout is initializing right now.
const INITIALIZING: usize = 1;
// The stdout has been initialized and currently is active.
const INITIALIZED: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetStdoutError(());

impl SetStdoutError {
    fn new() -> Self {
        Self(())
    }
}

struct NopOut;

impl StdOut for NopOut {
    fn write_str(&self, _: &str) -> fmt::Result {
        Ok(())
    }

    fn write_bytes(&self, _bytes: &[u8]) -> fmt::Result {
        Ok(())
    }

    fn write_fmt(&self, _args: fmt::Arguments) -> fmt::Result {
        Ok(())
    }

    fn flush(&self) -> fmt::Result {
        Ok(())
    }
}

fn set_stdout_inner<F>(stdout: F) -> Result<(), SetStdoutError>
where
    F: FnOnce() -> &'static dyn StdOut,
{
    let old_state = match STATE.compare_exchange(
        UNINITIALIZED,
        INITIALIZING,
        Ordering::SeqCst,
        Ordering::SeqCst,
    ) {
        Ok(s) | Err(s) => s,
    };

    match old_state {
        // The state was UNINITIALIZED and then changed to INITIALIZING.
        UNINITIALIZED => {
            unsafe {
                STDOUT = stdout();
            }
            STATE.store(INITIALIZED, Ordering::SeqCst);

            Ok(())
        }

        // The state is already INITIALIZING.
        INITIALIZING => {
            // Make sure the state became INITIALIZING finally.
            while STATE.load(Ordering::SeqCst) == INITIALIZING {
                hint::spin_loop();
            }

            Err(SetStdoutError::new())
        }

        _ => Err(SetStdoutError::new()),
    }
}

/// Initialized the global stdout with the a specified `&'static dyn StdOut`.
///
/// This function may only be called once during the program lifecycle.
pub fn init(stdout: &'static dyn StdOut) -> Result<(), SetStdoutError> {
    set_stdout_inner(move || stdout)
}

/// Returns a reference to the stdout.
///
/// If a stdout has not been set, returns a no-op implementation.
pub fn stdout() -> &'static dyn StdOut {
    if STATE.load(Ordering::SeqCst) != INITIALIZED {
        static NOP: NopOut = NopOut;
        &NOP
    } else {
        unsafe { STDOUT }
    }
}

/// Macro for printing to the configured stdout, without a newline.
#[macro_export]
macro_rules! uprint {
    ($s:expr) => {{
        $crate::stdout()
            .write_str($s)
            .ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::stdout()
            .write_fmt(format_args!($s, $($tt)*))
            .ok();
    }};
}

/// Macro for printing to the configured stdout, with a newline.
#[macro_export]
macro_rules! uprintln {
    () => {{
        $crate::stdout()
            .write_str(uprintln!(@newline))
            .ok();
    }};
    ($s:expr) => {{
        $crate::stdout()
            .write_str(concat!($s, uprintln!(@newline)))
            .ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::stdout()
            .write_fmt(format_args!(concat!($s, uprintln!(@newline)), $($tt)*))
            .ok();
    }};

    (@newline) => { "\r\n" };
}

/// Macro for printing to the configured stdout, without a newline.
///
/// This method prints only if the `dprint` feature enabled, which is useful
/// for debugging purposes.
#[cfg(any(feature = "dprint", doc))]
#[macro_export]
macro_rules! dprint {
    ($s:expr) => {{
        $crate::uprint!();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::uprint!($s, $($tt:tt)*);
    }};
}
#[cfg(not(any(feature = "dprint", doc)))]
#[macro_export]
macro_rules! dprint {
    ($s:expr) => {};
    ($s:expr, $($tt:tt)*) => {};
}

/// Macro for printing to the configured stdout, with a newline.
///
/// This method prints only if the `dprint` feature enabled, which is useful
/// for debugging purposes.
#[macro_export]
#[cfg(any(feature = "dprint", doc))]
macro_rules! dprintln {
    () => {{
        #[cfg(feature = "dprint")]
        $crate::uprintln!();
    }};
    ($s:expr) => {{
        #[cfg(feature = "dprint")]
        $crate::uprintln!($s);
    }};
    ($s:expr, $($tt:tt)*) => {{
        #[cfg(feature = "dprint")]
        $crate::uprintln!($s, $($tt:tt)*);
    }};
}
#[cfg(not(any(feature = "dprint", doc)))]
#[macro_export]
macro_rules! dprintln {
    () => {};
    ($s:expr) => {};
    ($s:expr, $($tt:tt)*) => {};
}
