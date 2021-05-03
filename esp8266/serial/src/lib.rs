#[cfg_attr(target_os = "none", no_std)]

#[cfg(not(target_os = "none"))]
pub mod serial;
