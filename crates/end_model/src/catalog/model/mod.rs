mod builder;
mod types;

pub use builder::CatalogBuilder;
pub use types::{
    FacilityDef, FacilityId, ItemDef, ItemId, PowerRecipe, PowerRecipeId, Recipe, RecipeId, Stack,
    ThermalBankDef,
};

use std::collections::HashMap;

use crate::Key;

/// Canonical in-memory model resolved from TOML inputs.
///
/// ## Design notes
///
/// This type intentionally keeps its fields private so that the internal indices (`key -> id`
/// lookups and the `thermal_bank` definition) cannot be desynchronized from the backing vectors by
/// accident.
///
/// Construct a catalog via [`Catalog::builder`] / [`CatalogBuilder::build`].
#[derive(Debug, Clone)]
pub struct Catalog {
    items: Vec<ItemDef>,
    facilities: Vec<FacilityDef>,
    recipes: Vec<Recipe>,
    power_recipes: Vec<PowerRecipe>,
    item_index: HashMap<Key, ItemId>,
    facility_index: HashMap<Key, FacilityId>,
    thermal_bank: ThermalBankDef,
}

/// Core power generation capacity in watts (fixed game constant).
const CORE_POWER_W: u32 = 200;

impl Catalog {
    /// Starts building a self-consistent [`Catalog`].
    pub fn builder() -> CatalogBuilder {
        CatalogBuilder::new()
    }

    /// Returns the core generation capacity in watts (fixed at 200W).
    pub fn core_power_w(&self) -> u32 {
        CORE_POWER_W
    }

    /// Returns item metadata by id.
    pub fn item(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.index())
    }

    /// Returns facility metadata by id.
    pub fn facility(&self, id: FacilityId) -> Option<&FacilityDef> {
        self.facilities.get(id.index())
    }

    /// Returns a recipe by its id.
    ///
    /// The optimizer and report layers use this id as a stable reference.
    pub fn recipe(&self, id: RecipeId) -> Option<&Recipe> {
        self.recipes.get(id.index())
    }

    /// Returns a power recipe by its id.
    pub fn power_recipe(&self, id: PowerRecipeId) -> Option<&PowerRecipe> {
        self.power_recipes.get(id.index())
    }

    /// Returns the unique thermal bank definition.
    pub fn thermal_bank(&self) -> &ThermalBankDef {
        &self.thermal_bank
    }

    /// Returns all items in id order.
    pub fn items(&self) -> &[ItemDef] {
        &self.items
    }

    /// Returns all facilities in id order.
    pub fn facilities(&self) -> &[FacilityDef] {
        &self.facilities
    }

    /// Returns all production recipes.
    pub fn recipes(&self) -> &[Recipe] {
        &self.recipes
    }

    /// Returns all production recipes paired with their stable ids.
    pub fn recipes_with_id(&self) -> impl Iterator<Item = (RecipeId, &Recipe)> + '_ {
        self.recipes
            .iter()
            .enumerate()
            .map(|(index, recipe)| (RecipeId::from_index(index), recipe))
    }

    /// Returns all production recipes paired with ids and already-resolved facility metadata.
    pub fn recipes_with_id_and_facility(
        &self,
    ) -> impl Iterator<Item = (RecipeId, &Recipe, &FacilityDef)> + '_ {
        self.recipes.iter().enumerate().map(|(index, recipe)| {
            // SAFETY: `CatalogBuilder::push_recipe` guarantees each `Recipe::facility` points to an
            // existing facility in this catalog, so the index is always in-bounds.
            let facility = unsafe { self.facilities.get_unchecked(recipe.facility.index()) };
            (RecipeId::from_index(index), recipe, facility)
        })
    }

    /// Returns all thermal-bank power recipes.
    pub fn power_recipes(&self) -> &[PowerRecipe] {
        &self.power_recipes
    }

    /// Returns all thermal-bank power recipes paired with their stable ids.
    pub fn power_recipes_with_id(
        &self,
    ) -> impl Iterator<Item = (PowerRecipeId, &PowerRecipe)> + '_ {
        self.power_recipes
            .iter()
            .enumerate()
            .map(|(index, recipe)| (PowerRecipeId::from_index(index), recipe))
    }

    /// Resolves an item key into an [`ItemId`].
    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.item_index.get(key).copied()
    }

    /// Resolves a facility key into a [`FacilityId`].
    pub fn facility_id(&self, key: &str) -> Option<FacilityId> {
        self.facility_index.get(key).copied()
    }
}
