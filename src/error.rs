use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SymseekError {
    #[error("File '{name}' not found in {searched_locations:?}")]
    NotFound {
        name: String,
        searched_locations: Vec<String>,
    },

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Failed to resolve symlink at {path:?}: {reason}")]
    SymlinkResolution { path: PathBuf, reason: String },

    #[error("Invalid path encoding: {path:?}")]
    PathEncoding { path: PathBuf },

    #[error("Cycle detected in chain at {path:?}")]
    CycleDetected { path: PathBuf },

    #[error("Failed to parse wrapper at {path:?}: {reason}")]
    WrapperParsing { path: PathBuf, reason: String },

    #[error("JSON serialization failed: {0}")]
    JsonSerialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SymseekError>;
