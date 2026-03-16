use crate::Key;

/// Errors returned when building a [`Catalog`](super::Catalog).
#[derive(Debug, thiserror::Error)]
pub enum CatalogBuildError {
    #[error("Duplicate item key: {0}")]
    DuplicateItemKey(Key),
    #[error("Duplicate facility key: {0}")]
    DuplicateFacilityKey(Key),
    #[error("Recipe ingredients contains duplicate item id {item_id}")]
    DuplicateRecipeIngredientItem { item_id: u32 },
    #[error("Recipe products contains duplicate item id {item_id}")]
    DuplicateRecipeProductItem { item_id: u32 },
    #[error("Recipe ingredients must not be empty")]
    RecipeIngredientsMustNotBeEmpty,
}
