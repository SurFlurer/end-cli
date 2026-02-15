mod error;
mod model;

pub use error::CatalogBuildError;
// Re-export catalog model. model is isolated to minimize exposure of Catalog ctor and Id ctors.
pub use model::{
    Catalog, CatalogBuilder, FacilityDef, FacilityId, ItemDef, ItemId, PowerRecipe, PowerRecipeId,
    Recipe, RecipeId, Stack, ThermalBankDef,
};

/// Core generation capacity (watts).
pub const P_CORE_W: u32 = 200;

#[cfg(test)]
mod tests;
