use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Catalog load failed: {0}")]
    Catalog(#[source] end_io::Error),

    #[error("Default aic generation failed: {0}")]
    DefaultAic(#[source] end_io::Error),

    #[error("Aic parse failed: {0}")]
    Aic(#[source] end_io::Error),

    #[error("Optimization failed: {0}")]
    Optimize(#[source] end_opt::Error),

    #[error("Missing outpost id {0:?}")]
    MissingOutpost(u32),

    #[error("Missing logistics node {node:?} for item {item:?}")]
    MissingLogisticsNode {
        item: u32,
        node: u32,
    },

    #[error("Unknown lang `{value}` (expected `zh` or `en`)")]
    UnknownLang { value: Box<str> },

    #[error("Null pointer for argument `{name}`")]
    NullPointer { name: &'static str },

    #[error("Argument `{name}` is not valid UTF-8")]
    InvalidUtf8 { name: &'static str },
}

pub type Result<T> = std::result::Result<T, Error>;
