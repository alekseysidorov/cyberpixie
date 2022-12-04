use displaydoc::Display;

/// Errors that can occur when processing messages.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Debug)]
pub enum Error {
    /// The length of the strip does not match with the specified.
    StripLengthMismatch,
    /// The length of the picture in bytes is not a multiple of "strip length" * "bytes per pixel".
    ImageLengthMismatch,
    /// The transmitted message cannot be fitted into the device's memory.
    ImageTooBig,
    /// This image repository on the device is full.
    ImageRepositoryFull,
    /// The specified image index is greater than the total amount of the stored images.
    ImageNotFound,
    /// Unexpected response to the request.
    UnexpectedResponse,
    /// Unspecified or unknown error.
    Unspecified(u16),
}

impl Error {
    pub(crate) fn from_code(code: u16) -> Self {
        match code {
            1 => Self::StripLengthMismatch,
            2 => Self::ImageLengthMismatch,
            3 => Self::ImageTooBig,
            4 => Self::ImageRepositoryFull,
            5 => Self::ImageNotFound,
            6 => Self::UnexpectedResponse,
            other => Self::Unspecified(other),
        }
    }

    pub(crate) fn into_code(self) -> u16 {
        match self {
            Error::StripLengthMismatch => 1,
            Error::ImageLengthMismatch => 2,
            Error::ImageTooBig => 3,
            Error::ImageRepositoryFull => 4,
            Error::ImageNotFound => 5,
            Error::UnexpectedResponse => 6,
            Error::Unspecified(other) => other,
        }
    }
}
