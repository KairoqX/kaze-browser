//! Shared error type used across Kaze crates.
//!
//! Individual crates may define their own narrower error enums, but
//! anything that crosses a crate boundary into `kaze-app` should be
//! representable as a [`KazeError`] (usually via `#[from]`).

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum KazeError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse config: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("failed to serialize config: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("could not resolve platform directories (no HOME?)")]
    NoPlatformDirs,

    #[error("database error: {0}")]
    Database(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, KazeError>;

impl KazeError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
