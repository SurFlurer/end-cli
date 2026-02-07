use std::path::PathBuf;
use thiserror::Error;

/// Result alias for conversion operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised while converting v1 Lua recipes into v2 TOML files.
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

    #[error("lua error in {path}: {source}")]
    Lua {
        path: PathBuf,
        #[source]
        source: mlua::Error,
    },

    #[error("schema error in {path}: {message}")]
    Schema { path: PathBuf, message: String },

    #[error("missing zh translation for {kind} `{key}`")]
    MissingI18n { kind: &'static str, key: String },

    #[error("missing power_w for facility `{facility}` in facility_power.toml")]
    MissingFacilityPower { facility: String },
}
