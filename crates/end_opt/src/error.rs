use good_lp::ResolutionError;
use thiserror::Error;

/// Result alias for optimizer operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised while building or solving optimization models.
#[derive(Debug, Error)]
pub enum Error {
    #[error("solver failed: {source}")]
    Solver {
        #[source]
        source: ResolutionError,
    },

    #[error("invalid input: {message}")]
    InvalidInput { message: String },

    #[error("missing machine power for facility `{facility}`")]
    MissingMachinePower { facility: String },

    #[error(
        "value not near integer for `{var_name}`: value={value}, nearest={nearest}, delta={delta}, eps={eps}"
    )]
    NotNearInt {
        var_name: String,
        value: f64,
        nearest: f64,
        delta: f64,
        eps: f64,
    },

    #[error("value out of range for `{var_name}`: {value}")]
    OutOfRange { var_name: String, value: f64 },

    #[error("internal error: {message}")]
    Internal { message: String },
}
