use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::num::NonZeroU32;

use generativity::{Guard, Id};

use crate::Key;

use super::super::CatalogBuildError;
use super::{
    Catalog, FacilityDef, FacilityId, ItemDef, ItemId, PowerRecipe, PowerRecipeId, Recipe,
    RecipeId, Stack, ThermalBankDef,
};

/// Marker state: thermal bank has not been provided yet.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThermalBankMissing;

/// Marker state: thermal bank has been provided and is ready for `build`.
#[derive(Debug, Clone)]
pub struct ThermalBankReady(ThermalBankDef);

impl ThermalBankReady {
    fn into_inner(self) -> ThermalBankDef {
        self.0
    }
}

pub trait ThermalBankState {
    fn thermal_bank(&self) -> Option<&ThermalBankDef>;
}

impl ThermalBankState for ThermalBankMissing {
    fn thermal_bank(&self) -> Option<&ThermalBankDef> {
        None
    }
}

impl ThermalBankState for ThermalBankReady {
    fn thermal_bank(&self) -> Option<&ThermalBankDef> {
        Some(&self.0)
    }
}

/// Builder for [`Catalog`].
///
/// This is the only supported way to construct a catalog, because it:
/// - Assigns catalog-dependent ids (`ItemId`/`FacilityId`) in a consistent order.
/// - Maintains key->id indices (`item_index`, `facility_index`).
/// - Enforces catalog-level invariants.
#[derive(Debug)]
pub struct CatalogBuilder<'id, State = ThermalBankMissing> {
    brand: Id<'id>,
    items: Vec<ItemDef>,
    facilities: Vec<FacilityDef>,
    recipes: Vec<Recipe<'id>>,
    power_recipes: Vec<PowerRecipe<'id>>,
    item_index: HashMap<Key, ItemId<'id>>,
    facility_index: HashMap<Key, FacilityId<'id>>,
    thermal_bank: State,
}

impl<'id> CatalogBuilder<'id, ThermalBankMissing> {
    /// Creates an empty builder.
    pub fn new(guard: Guard<'id>) -> Self {
        Self {
            brand: guard.into(),
            items: Vec::new(),
            facilities: Vec::new(),
            recipes: Vec::new(),
            power_recipes: Vec::new(),
            item_index: HashMap::new(),
            facility_index: HashMap::new(),
            thermal_bank: ThermalBankMissing,
        }
    }

    /// Adds the unique thermal bank definition and transitions builder state.
    pub fn add_thermal_bank(
        self,
        def: ThermalBankDef,
    ) -> Result<CatalogBuilder<'id, ThermalBankReady>, CatalogBuildError> {
        if self.facility_index.contains_key(def.key.as_str()) {
            return Err(CatalogBuildError::DuplicateFacilityKey(def.key));
        }
        let Self {
            brand,
            items,
            facilities,
            recipes,
            power_recipes,
            item_index,
            facility_index,
            thermal_bank: _,
        } = self;
        Ok(CatalogBuilder {
            brand,
            items,
            facilities,
            recipes,
            power_recipes,
            item_index,
            facility_index,
            thermal_bank: ThermalBankReady(def),
        })
    }
}

impl<'id, State: ThermalBankState> CatalogBuilder<'id, State> {
    /// Adds an item definition and returns its newly assigned [`ItemId`].
    ///
    /// Item keys must be unique.
    pub fn add_item(&mut self, def: ItemDef) -> Result<ItemId<'id>, CatalogBuildError> {
        match self.item_index.entry(def.key.clone()) {
            Entry::Occupied(_) => Err(CatalogBuildError::DuplicateItemKey(def.key)),
            Entry::Vacant(slot) => {
                let id = ItemId::from_index(self.items.len(), self.brand);
                slot.insert(id);
                self.items.push(def);
                Ok(id)
            }
        }
    }

    /// Adds a facility definition and returns its newly assigned [`FacilityId`].
    ///
    /// Facility keys must be unique.
    pub fn add_facility(&mut self, def: FacilityDef) -> Result<FacilityId<'id>, CatalogBuildError> {
        if self
            .thermal_bank
            .thermal_bank()
            .is_some_and(|bank| bank.key.as_str() == def.key.as_str())
        {
            return Err(CatalogBuildError::DuplicateFacilityKey(def.key));
        }

        match self.facility_index.entry(def.key.clone()) {
            Entry::Occupied(_) => Err(CatalogBuildError::DuplicateFacilityKey(def.key)),
            Entry::Vacant(slot) => {
                let id = FacilityId::from_index(self.facilities.len(), self.brand);
                slot.insert(id);
                self.facilities.push(def);
                Ok(id)
            }
        }
    }

    /// Appends a production recipe and returns its assigned [`RecipeId`].
    ///
    /// This validates recipe-local invariants and cross-references against the current
    /// builder state:
    /// - `time_s` is positive by type (`NonZeroU32`).
    /// - `products` must be non-empty.
    /// - each referenced item must exist.
    /// - each stack count is positive by type (`NonZeroU32`).
    /// - duplicate items are rejected within `ingredients` and `products` respectively.
    pub fn push_recipe(
        &mut self,
        facility: FacilityId<'id>,
        time_s: NonZeroU32,
        ingredients: Box<[Stack<'id>]>,
        products: Box<[Stack<'id>]>,
    ) -> Result<RecipeId<'id>, CatalogBuildError> {
        self.validate_recipe_stacks(&ingredients, |item_id| {
            CatalogBuildError::DuplicateRecipeIngredientItem { item_id }
        })?;
        self.validate_recipe_stacks(&products, |item_id| {
            CatalogBuildError::DuplicateRecipeProductItem { item_id }
        })?;

        let id = RecipeId::from_index(self.recipes.len(), self.brand);
        self.recipes.push(Recipe {
            facility,
            time_s: time_s.get(),
            ingredients,
            products,
        });
        Ok(id)
    }

    /// Appends a thermal-bank power recipe and returns its assigned [`PowerRecipeId`].
    ///
    /// Positive-value invariants are enforced by type (`NonZeroU32`):
    /// - `ingredient.count`
    /// - `power_w`
    /// - `time_s`
    pub fn push_power_recipe(
        &mut self,
        recipe: PowerRecipe<'id>,
    ) -> Result<PowerRecipeId<'id>, CatalogBuildError> {
        let id = PowerRecipeId::from_index(self.power_recipes.len(), self.brand);
        self.power_recipes.push(recipe);
        Ok(id)
    }

    /// Resolves an item key into an [`ItemId`] using the current builder state.
    pub fn item_id(&self, key: &str) -> Option<ItemId<'id>> {
        self.item_index.get(key).copied()
    }

    /// Resolves a facility key into a [`FacilityId`] using the current builder state.
    pub fn facility_id(&self, key: &str) -> Option<FacilityId<'id>> {
        self.facility_index.get(key).copied()
    }

    fn validate_recipe_stacks(
        &self,
        stacks: &[Stack<'id>],
        on_duplicate_item: impl Fn(u32) -> CatalogBuildError,
    ) -> Result<(), CatalogBuildError> {
        let mut seen = HashSet::with_capacity(stacks.len());
        for stack in stacks {
            if !seen.insert(stack.item) {
                return Err(on_duplicate_item(stack.item.as_u32()));
            }
        }
        Ok(())
    }
}

impl<'id> CatalogBuilder<'id, ThermalBankReady> {
    /// Finalizes the builder and returns a self-consistent [`Catalog`].
    pub fn build(self) -> Catalog<'id> {
        let Self {
            brand,
            items,
            facilities,
            recipes,
            power_recipes,
            item_index,
            facility_index,
            thermal_bank,
        } = self;
        Catalog {
            brand,
            items: items.into_boxed_slice(),
            facilities: facilities.into_boxed_slice(),
            recipes: recipes.into_boxed_slice(),
            power_recipes: power_recipes.into_boxed_slice(),
            item_index,
            facility_index,
            thermal_bank: thermal_bank.into_inner(),
        }
    }
}
