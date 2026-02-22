use thiserror::Error;

/// Result alias for report generation.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised when report rendering references missing input/logistics indices.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing outpost index {}", .0)]
    MissingOutpost(u32),

    #[error("Missing logistics node {node:?} for item id {item:?}")]
    MissingLogisticsNode { item: u32, node: u32 },
}
