use std::path::PathBuf;
use thiserror::Error;

/// Result alias for IO and schema-loading operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised while loading/validating TOML inputs.
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize TOML: {source}")]
    TomlSerialize {
        #[source]
        source: toml::ser::Error,
    },

    #[error("schema error in {path}, field `{field}`, index={index:?}: {message}")]
    Schema {
        path: PathBuf,
        field: String,
        index: Option<usize>,
        message: String,
    },

    #[error("duplicate {kind} key `{key}` in {path}")]
    DuplicateKey {
        path: PathBuf,
        kind: String,
        key: String,
    },

    #[error("unknown item `{key}` in {path}")]
    UnknownItem { path: PathBuf, key: String },

    #[error("unknown facility `{key}` in {path}")]
    UnknownFacility { path: PathBuf, key: String },
}
