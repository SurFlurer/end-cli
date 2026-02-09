use crate::{Error, Result};
use std::num::NonZeroU32;
use std::path::Path;

pub(crate) fn parse_positive_u32(path: &Path, field: String, value: i64) -> Result<NonZeroU32> {
    if value < 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} must be >= 1, got {value}"),
        });
    }
    let parsed = u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        message: format!("{field} out of range for u32: {value}"),
    })?;
    NonZeroU32::new(parsed).ok_or_else(|| Error::Schema {
        path: path.to_path_buf(),
        message: format!("{field} must be >= 1, got {value}"),
    })
}

pub(crate) fn parse_positive_u32_from_f64(
    path: &Path,
    field: String,
    value: f64,
) -> Result<NonZeroU32> {
    if !value.is_finite() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} must be finite, got {value}"),
        });
    }

    let nearest = value.round();
    let delta = (value - nearest).abs();
    if delta > 1e-9 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!(
                "{field} must be integer seconds, got {value} (nearest {nearest}, delta {delta})"
            ),
        });
    }

    if nearest < 1.0 || nearest > u32::MAX as f64 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!("{field} out of range for u32: {value}"),
        });
    }

    NonZeroU32::new(nearest as u32).ok_or_else(|| Error::Schema {
        path: path.to_path_buf(),
        message: format!("{field} must be >= 1, got {value}"),
    })
}
