pub use stdio_serial::init;

/// Macro for printing to the specified output, without a newline.
#[macro_export]
macro_rules! uwrite {
    ($o:expr, $s:expr) => {{
        use core::fmt::Write;
        $o.write_str($s).ok();
    }};
    ($o:expr, $s:expr, $($tt:tt)*) => {{
        use core::fmt::Write;
        $o.write_fmt(format_args!($s, $($tt)*)).ok();
    }};
}

/// Macro for printing to the specified output, with a newline.
#[macro_export]
macro_rules! uwriteln {
    ($o:expr) => {{
        use core::fmt::Write;
        $o.write_str("\r\n").ok();
    }};
    ($o:expr, $s:expr) => {{
        use core::fmt::Write;
        $o.write_str(concat!($s, "\r\n")).ok();
    }};
    ($o:expr, $s:expr, $($tt:tt)*) => {{
        use core::fmt::Write;
        $o.write_fmt(format_args!(concat!($s, "\r\n"), $($tt)*)).ok();
    }};
}
