use end_model::{FacilityId, ItemId};
use end_opt::{OutpostIndex, RecipeIndex};
use thiserror::Error;

/// Result alias for report generation.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised when report rendering references missing catalog/input indices.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing item id {0:?}")]
    MissingItem(ItemId),

    #[error("missing facility id {0:?}")]
    MissingFacility(FacilityId),

    #[error("missing outpost index {}", .0.as_u32())]
    MissingOutpost(OutpostIndex),

    #[error("missing recipe index {}", .0.as_u32())]
    MissingRecipe(RecipeIndex),
}
