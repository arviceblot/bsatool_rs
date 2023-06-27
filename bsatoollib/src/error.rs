//! BSA related errors
use thiserror::Error;

/// BSA Error types
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum BsaError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("BSA file must be open before reading")]
    NotOpen,
    #[error("BSA file is already open for reading")]
    AlreadyOpen,
    #[error("File too small to be a valid BSA: {0} bytes")]
    TooSmall(u64),
    #[error("Unrecognized BSA header")]
    BadHeader,
    #[error("Directory information larger than entire archive, file may be corrupt")]
    DirSize,
    #[error("Archive contains offsets outside itself")]
    OffsetOutside,
    #[error("File not found in archive: {0}")]
    FileNotFound(String),
    #[error("Read file positions should be {expected} but was {actual}")]
    Position { expected: u32, actual: u64 },
    #[error("Expected to write {expected} bytes but was {actual}")]
    BytesWritten { expected: u32, actual: usize },
}

/// Result type using BsaErrors
pub type Result<T, E = BsaError> = std::result::Result<T, E>;
