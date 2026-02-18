use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FilesystemError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Query parse error: {0}")]
    QueryParse(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("File too large: {0}")]
    FileTooLarge(PathBuf),

    #[error("Not a text file: {0}")]
    NotTextFile(PathBuf),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, FilesystemError>;

/// Canonicalizes a path, returning the original if canonicalization fails.
pub fn canonicalize_existing_path(path: PathBuf) -> PathBuf {
    fs::canonicalize(&path).unwrap_or(path)
}
