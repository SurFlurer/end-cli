use std::collections::{HashMap, HashSet};

use crate::Key;

use super::super::CatalogBuildError;
use super::{
    Catalog, FacilityDef, FacilityId, ItemDef, ItemId, PowerRecipe, PowerRecipeId, Recipe,
    RecipeId, Stack, ThermalBankDef,
};

/// Builder for [`Catalog`].
///
/// This is the only supported way to construct a catalog, because it:
/// - Assigns catalog-dependent ids (`ItemId`/`FacilityId`) in a consistent order.
/// - Maintains key->id indices (`item_index`, `facility_index`).
/// - Enforces catalog-level invariants.
#[derive(Debug, Default)]
pub struct CatalogBuilder {
    items: Vec<ItemDef>,
    facilities: Vec<FacilityDef>,
    recipes: Vec<Recipe>,
    power_recipes: Vec<PowerRecipe>,
    item_index: HashMap<Key, ItemId>,
    facility_index: HashMap<Key, FacilityId>,
    thermal_bank: Option<ThermalBankDef>,
}

impl CatalogBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item definition and returns its newly assigned [`ItemId`].
    ///
    /// Item keys must be unique.
    pub fn add_item(&mut self, def: ItemDef) -> Result<ItemId, CatalogBuildError> {
        if self.item_index.contains_key(def.key.as_str()) {
            return Err(CatalogBuildError::DuplicateItemKey(def.key));
        }
        let id = ItemId::from_index(self.items.len());
        self.item_index.insert(def.key.clone(), id);
        self.items.push(def);
        Ok(id)
    }

    /// Adds a facility definition and returns its newly assigned [`FacilityId`].
    ///
    /// Facility keys must be unique.
    pub fn add_facility(&mut self, def: FacilityDef) -> Result<FacilityId, CatalogBuildError> {
        if self.facility_index.contains_key(def.key.as_str())
            || self
                .thermal_bank
                .as_ref()
                .is_some_and(|bank| bank.key.as_str() == def.key.as_str())
        {
            return Err(CatalogBuildError::DuplicateFacilityKey(def.key));
        }
        let id = FacilityId::from_index(self.facilities.len());
        self.facility_index.insert(def.key.clone(), id);
        self.facilities.push(def);
        Ok(id)
    }

    /// Adds the unique thermal bank definition.
    pub fn add_thermal_bank(&mut self, def: ThermalBankDef) -> Result<(), CatalogBuildError> {
        if self.thermal_bank.is_some() {
            return Err(CatalogBuildError::MultipleThermalBanks);
        }
        if self.facility_index.contains_key(def.key.as_str()) {
            return Err(CatalogBuildError::DuplicateFacilityKey(def.key));
        }
        self.thermal_bank = Some(def);
        Ok(())
    }

    /// Appends a production recipe and returns its assigned [`RecipeId`].
    ///
    /// This validates recipe-local invariants and cross-references against the current
    /// builder state:
    /// - `facility` must exist.
    /// - `time_s` must be positive.
    /// - `ingredients` / `products` must be non-empty.
    /// - each referenced item must exist.
    /// - duplicate items are rejected within `ingredients` and `products` respectively.
    pub fn push_recipe(
        &mut self,
        facility: FacilityId,
        time_s: u32,
        ingredients: Vec<Stack>,
        products: Vec<Stack>,
    ) -> Result<RecipeId, CatalogBuildError> {
        if self.facilities.get(facility.index()).is_none() {
            return Err(CatalogBuildError::UnknownRecipeFacilityId(
                facility.as_u32(),
            ));
        }
        if time_s == 0 {
            return Err(CatalogBuildError::RecipeTimeMustBePositive);
        }
        if ingredients.is_empty() {
            return Err(CatalogBuildError::RecipeIngredientsMustNotBeEmpty);
        }
        if products.is_empty() {
            return Err(CatalogBuildError::RecipeProductsMustNotBeEmpty);
        }
        self.validate_recipe_stacks("ingredients", &ingredients)?;
        self.validate_recipe_stacks("products", &products)?;

        let id = RecipeId::from_index(self.recipes.len());
        self.recipes.push(Recipe {
            facility,
            time_s,
            ingredients,
            products,
        });
        Ok(id)
    }

    /// Appends a thermal-bank power recipe and returns its assigned [`PowerRecipeId`].
    ///
    /// This validates power-recipe invariants and cross-references against current items:
    /// - `ingredient.item` must exist.
    /// - `ingredient.count` must be positive.
    /// - `power_w` must be positive.
    /// - `time_s` must be positive.
    pub fn push_power_recipe(
        &mut self,
        recipe: PowerRecipe,
    ) -> Result<PowerRecipeId, CatalogBuildError> {
        if self.items.get(recipe.ingredient.item.index()).is_none() {
            return Err(CatalogBuildError::UnknownPowerRecipeIngredientItemId(
                recipe.ingredient.item.as_u32(),
            ));
        }
        if recipe.ingredient.count == 0 {
            return Err(
                CatalogBuildError::PowerRecipeIngredientCountMustBePositive {
                    item_id: recipe.ingredient.item.as_u32(),
                },
            );
        }
        if recipe.power_w == 0 {
            return Err(CatalogBuildError::PowerRecipePowerMustBePositive);
        }
        if recipe.time_s == 0 {
            return Err(CatalogBuildError::PowerRecipeTimeMustBePositive);
        }

        let id = PowerRecipeId::from_index(self.power_recipes.len());
        self.power_recipes.push(recipe);
        Ok(id)
    }

    /// Resolves an item key into an [`ItemId`] using the current builder state.
    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.item_index.get(key).copied()
    }

    /// Resolves a facility key into a [`FacilityId`] using the current builder state.
    pub fn facility_id(&self, key: &str) -> Option<FacilityId> {
        self.facility_index.get(key).copied()
    }

    /// Returns the thermal bank definition if it has already been added.
    pub fn thermal_bank(&self) -> Option<&ThermalBankDef> {
        self.thermal_bank.as_ref()
    }

    /// Finalizes the builder and returns a self-consistent [`Catalog`].
    ///
    /// Fails if required invariants are not met (e.g. thermal bank missing).
    pub fn build(self) -> Result<Catalog, CatalogBuildError> {
        let thermal_bank = self
            .thermal_bank
            .ok_or(CatalogBuildError::MissingThermalBank)?;
        Ok(Catalog {
            items: self.items,
            facilities: self.facilities,
            recipes: self.recipes,
            power_recipes: self.power_recipes,
            item_index: self.item_index,
            facility_index: self.facility_index,
            thermal_bank,
        })
    }

    fn validate_recipe_stacks(
        &self,
        list: &'static str,
        stacks: &[Stack],
    ) -> Result<(), CatalogBuildError> {
        let mut seen = HashSet::with_capacity(stacks.len());
        for stack in stacks {
            if stack.count == 0 {
                return Err(CatalogBuildError::RecipeStackCountMustBePositive {
                    list,
                    item_id: stack.item.as_u32(),
                });
            }
            if self.items.get(stack.item.index()).is_none() {
                return Err(CatalogBuildError::UnknownRecipeItemId {
                    list,
                    item_id: stack.item.as_u32(),
                });
            }
            if !seen.insert(stack.item) {
                return Err(CatalogBuildError::DuplicateRecipeItem {
                    list,
                    item_id: stack.item.as_u32(),
                });
            }
        }
        Ok(())
    }
}
