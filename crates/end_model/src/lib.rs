use std::collections::HashMap;
use std::iter::FromIterator;
use std::num::NonZeroU32;
use vector_map::VecMap;

/// Base/core generation capacity (watts) used by the default CLI flow.
pub const P_CORE_W: u32 = 200;

/// Stable identifier for an item in [`Catalog`].
///
/// This is an *opaque* id: external callers cannot construct it from a raw `u32`.
/// IDs are minted by [`CatalogBuilder`] when building a self-consistent [`Catalog`].
///
/// Note: an `ItemId` is only meaningful *relative to a specific* [`Catalog`] instance.
/// Even if you can obtain the underlying number (via [`ItemId::as_u32`]), that does
/// **not** mean it is valid in another catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u32);

impl ItemId {
    /// Returns the underlying numeric representation.
    ///
    /// In a single [`Catalog`], item ids are minted densely in insertion order, so this
    /// value can be used as an index (`as_u32() as usize`) for per-item arrays whose
    /// length is `catalog.items().len()`.
    ///
    /// This is still an opaque id boundary: even though callers can read the number,
    /// it does **not** make the id safe to construct externally, and ids from different
    /// catalogs must not be mixed.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Stable identifier for a facility in [`Catalog`].
///
/// Like [`ItemId`], this is an opaque id minted by [`CatalogBuilder`], and it is only
/// meaningful relative to the catalog it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FacilityId(u32);

impl FacilityId {
    /// Returns the underlying numeric representation.
    ///
    /// In a single [`Catalog`], facility ids are minted densely in insertion order, so
    /// this value can be used as an index (`as_u32() as usize`) for per-facility arrays
    /// whose length is `catalog.facilities().len()`.
    ///
    /// As with [`ItemId`], ids are catalog-scoped and must not be mixed across catalogs.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Facility category used by the optimizer and report layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FacilityKind {
    Machine,
    ThermalBank,
}

/// Item metadata and display texts.
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub key: String,
    pub en: String,
    pub zh: String,
}

/// Facility metadata and display texts.
#[derive(Debug, Clone)]
pub struct FacilityDef {
    pub key: String,
    pub kind: FacilityKind,
    pub power_w: Option<NonZeroU32>,
    pub en: String,
    pub zh: String,
}

/// `(item, count)` pair used in recipes.
#[derive(Debug, Clone, Copy)]
pub struct Stack {
    pub item: ItemId,
    pub count: u32,
}

/// Production recipe definition.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub facility: FacilityId,
    pub time_s: u32,
    pub ingredients: Vec<Stack>,
    pub products: Vec<Stack>,
}

/// Thermal-bank power recipe definition.
#[derive(Debug, Clone, Copy)]
pub struct PowerRecipe {
    pub ingredient: Stack,
    pub power_w: u32,
    pub time_s: u32,
}

/// Canonical in-memory model resolved from TOML inputs.
///
/// ## Design notes
///
/// This type intentionally keeps its fields private so that the internal indices
/// (`key -> id` lookups and the `thermal_bank` id) cannot be desynchronized from the
/// backing vectors by accident.
///
/// Construct a catalog via [`Catalog::builder`] / [`CatalogBuilder::build`].
#[derive(Debug, Clone)]
pub struct Catalog {
    items: Vec<ItemDef>,
    facilities: Vec<FacilityDef>,
    recipes: Vec<Recipe>,
    power_recipes: Vec<PowerRecipe>,
    item_index: HashMap<String, ItemId>,
    facility_index: HashMap<String, FacilityId>,
    thermal_bank: FacilityId,
}

impl Catalog {
    /// Starts building a self-consistent [`Catalog`].
    pub fn builder() -> CatalogBuilder {
        CatalogBuilder::new()
    }

    /// Returns item metadata by id.
    pub fn item(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.index())
    }

    /// Returns facility metadata by id.
    pub fn facility(&self, id: FacilityId) -> Option<&FacilityDef> {
        self.facilities.get(id.index())
    }

    /// Returns a recipe by its index in [`Catalog::recipes`].
    ///
    /// The optimizer and report layers use this index as a stable reference.
    pub fn recipe(&self, index: usize) -> Option<&Recipe> {
        self.recipes.get(index)
    }

    /// Returns a power recipe by its index in [`Catalog::power_recipes`].
    pub fn power_recipe(&self, index: usize) -> Option<&PowerRecipe> {
        self.power_recipes.get(index)
    }

    /// Returns the facility id of the unique thermal bank facility.
    pub fn thermal_bank(&self) -> FacilityId {
        self.thermal_bank
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

    /// Returns all thermal-bank power recipes.
    pub fn power_recipes(&self) -> &[PowerRecipe] {
        &self.power_recipes
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

/// Builder for [`Catalog`].
///
/// This is the only supported way to construct a catalog, because it:
/// - Mints opaque ids (`ItemId`/`FacilityId`) in a consistent order.
/// - Maintains key->id indices (`item_index`, `facility_index`).
/// - Enforces catalog-level invariants.
#[derive(Debug, Default)]
pub struct CatalogBuilder {
    items: Vec<ItemDef>,
    facilities: Vec<FacilityDef>,
    recipes: Vec<Recipe>,
    power_recipes: Vec<PowerRecipe>,
    item_index: HashMap<String, ItemId>,
    facility_index: HashMap<String, FacilityId>,
    thermal_bank: Option<FacilityId>,
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
    /// Facility keys must be unique. Exactly one facility with kind
    /// [`FacilityKind::ThermalBank`] is required to build a [`Catalog`].
    pub fn add_facility(&mut self, def: FacilityDef) -> Result<FacilityId, CatalogBuildError> {
        if self.facility_index.contains_key(def.key.as_str()) {
            return Err(CatalogBuildError::DuplicateFacilityKey(def.key));
        }
        let id = FacilityId::from_index(self.facilities.len());
        self.facility_index.insert(def.key.clone(), id);
        if def.kind == FacilityKind::ThermalBank {
            if self.thermal_bank.is_some() {
                return Err(CatalogBuildError::MultipleThermalBanks);
            }
            self.thermal_bank = Some(id);
        }
        self.facilities.push(def);
        Ok(id)
    }

    /// Appends a production recipe.
    ///
    /// This does not perform deep validation of cross-references; callers are expected
    /// to resolve ids via [`CatalogBuilder::item_id`] / [`CatalogBuilder::facility_id`]
    /// before constructing recipes.
    pub fn push_recipe(&mut self, recipe: Recipe) {
        self.recipes.push(recipe);
    }

    /// Appends a thermal-bank power recipe.
    pub fn push_power_recipe(&mut self, recipe: PowerRecipe) {
        self.power_recipes.push(recipe);
    }

    /// Resolves an item key into an [`ItemId`] using the current builder state.
    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.item_index.get(key).copied()
    }

    /// Resolves a facility key into a [`FacilityId`] using the current builder state.
    pub fn facility_id(&self, key: &str) -> Option<FacilityId> {
        self.facility_index.get(key).copied()
    }

    /// Returns the thermal bank id if it has already been added.
    pub fn thermal_bank(&self) -> Option<FacilityId> {
        self.thermal_bank
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
}

/// Errors returned when building a [`Catalog`].
#[derive(Debug, thiserror::Error)]
pub enum CatalogBuildError {
    #[error("duplicate item key: {0}")]
    DuplicateItemKey(String),
    #[error("duplicate facility key: {0}")]
    DuplicateFacilityKey(String),
    #[error("missing thermal bank facility")]
    MissingThermalBank,
    #[error("multiple thermal banks are not allowed")]
    MultipleThermalBanks,
}

/// Sparse map keyed by [`ItemId`] with unique keys guaranteed by representation.
///
/// This uses a vector-backed map (`VecMap`) and is intended for small collections
/// such as scenario supply and outpost price tables.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemU32Map(VecMap<ItemId, u32>);

impl ItemU32Map {
    pub fn new() -> Self {
        Self(VecMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(VecMap::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, item: ItemId, value: u32) -> Option<u32> {
        self.0.insert(item, value)
    }

    pub fn get(&self, item: ItemId) -> Option<&u32> {
        self.0.get(&item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ItemId, u32)> + '_ {
        self.0.iter().map(|(item, value)| (*item, *value))
    }
}

impl Extend<(ItemId, u32)> for ItemU32Map {
    fn extend<T: IntoIterator<Item = (ItemId, u32)>>(&mut self, iter: T) {
        for (item, value) in iter {
            self.insert(item, value);
        }
    }
}

impl FromIterator<(ItemId, u32)> for ItemU32Map {
    fn from_iter<T: IntoIterator<Item = (ItemId, u32)>>(iter: T) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

impl<const N: usize> From<[(ItemId, u32); N]> for ItemU32Map {
    fn from(value: [(ItemId, u32); N]) -> Self {
        value.into_iter().collect()
    }
}

impl From<Vec<(ItemId, u32)>> for ItemU32Map {
    fn from(value: Vec<(ItemId, u32)>) -> Self {
        value.into_iter().collect()
    }
}

impl IntoIterator for ItemU32Map {
    type Item = (ItemId, u32);
    type IntoIter = vector_map::IntoIter<ItemId, u32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// One outpost demand/cap configuration.
#[derive(Debug, Clone)]
pub struct OutpostInput {
    pub key: String,
    pub en: Option<String>,
    pub zh: Option<String>,
    pub money_cap_per_hour: u32,
    /// Sparse per-item price table.
    pub prices: ItemU32Map,
}

/// Full scenario inputs consumed by optimization.
#[derive(Debug, Clone)]
pub struct AicInputs {
    pub external_power_consumption_w: u32,
    /// Sparse per-item external supply table.
    pub supply_per_min: ItemU32Map,
    pub outposts: Vec<OutpostInput>,
}

#[cfg(test)]
mod tests {
    use super::{ItemId, ItemU32Map};

    #[test]
    fn item_u32_map_keeps_unique_keys() {
        let item = ItemId::from_index(7);
        let mut map = ItemU32Map::new();

        assert_eq!(map.insert(item, 10), None);
        assert_eq!(map.insert(item, 20), Some(10));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(item), Some(&20));
    }

    #[test]
    fn item_u32_map_from_vec_uses_last_value_for_duplicates() {
        let a = ItemId::from_index(1);
        let b = ItemId::from_index(2);
        let map: ItemU32Map = vec![(a, 1), (b, 3), (a, 2)].into();

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(a), Some(&2));
        assert_eq!(map.get(b), Some(&3));
    }
}
