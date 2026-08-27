use std::io;

use thiserror::Error;

use crate::geometry::GeometryError;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unsupported raster format{extension}")]
    UnsupportedFormat { extension: String },

    #[error("could not access the source image during {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("could not decode the source image: {message}")]
    Decode { message: String },

    #[error("image dimensions are invalid: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("image allocation would exceed the safety limit: {width}x{height}")]
    AllocationTooLarge { width: u32, height: u32 },

    #[error("invalid document quadrilateral: {0}")]
    Geometry(#[from] GeometryError),

    #[error("invalid enhancement parameter `{field}`: {reason}")]
    InvalidParameter { field: &'static str, reason: String },

    #[error("invalid output dimensions: {width}x{height}")]
    InvalidOutputDimensions { width: u32, height: u32 },

    #[error("could not encode the output image: {message}")]
    Encode { message: String },

    #[error("the output file already exists")]
    OutputExists,

    #[error(
        "could not atomically write the output image during {operation}: {source}"
    )]
    ExportIo {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    #[error(
        "session schema version {found} is not supported; expected {expected}"
    )]
    UnsupportedSessionSchema { found: u32, expected: u32 },

    #[error("could not serialize the session: {message}")]
    SessionEncode { message: String },

    #[error("could not parse the session: {message}")]
    SessionDecode { message: String },
}

impl CoreError {
    pub(crate) fn io(operation: &'static str, source: io::Error) -> Self {
        Self::Io { operation, source }
    }

    pub(crate) fn export_io(
        operation: &'static str,
        source: io::Error,
    ) -> Self {
        Self::ExportIo { operation, source }
    }
}
