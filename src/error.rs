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
}

pub type Result<T> = std::result::Result<T, SymseekError>;
