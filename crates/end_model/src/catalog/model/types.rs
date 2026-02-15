use std::num::NonZeroU32;

use crate::{DisplayName, Key};

/// Stable identifier for an item in [`Catalog`](super::Catalog).
///
/// `ItemId` is catalog-dependent: it is only meaningful for the specific
/// [`Catalog`](super::Catalog) instance that created it.
///
/// IDs are assigned during catalog construction via
/// [`Catalog::builder`](super::Catalog::builder) / [`CatalogBuilder`](super::CatalogBuilder).
///
/// Note: an `ItemId` is only meaningful *relative to a specific* [`Catalog`](super::Catalog)
/// instance. Even if you can obtain the underlying number (via [`ItemId::as_u32`]), that does
/// **not** mean it is valid in another catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u32);

impl ItemId {
    /// Returns the underlying numeric representation.
    ///
    /// In a single [`Catalog`](super::Catalog), item ids are minted densely in insertion order,
    /// so this value can be used as an index (via [`ItemId::index`]) for per-item arrays whose
    /// length is `catalog.items().len()`.
    ///
    /// The numeric value is catalog-dependent; ids from different catalogs must not be mixed.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the zero-based dense index of this item id in its source catalog.
    ///
    /// This is equivalent to `self.as_u32() as usize`, provided as a dedicated API to make
    /// per-item indexing more explicit.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }
}

/// Stable identifier for a facility in [`Catalog`](super::Catalog).
///
/// Like [`ItemId`], `FacilityId` is catalog-dependent and only meaningful relative to the catalog
/// that created it.
///
/// IDs are assigned during catalog construction via
/// [`Catalog::builder`](super::Catalog::builder) / [`CatalogBuilder`](super::CatalogBuilder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FacilityId(u32);

impl FacilityId {
    /// Returns the underlying numeric representation.
    ///
    /// In a single [`Catalog`](super::Catalog), facility ids are minted densely in insertion
    /// order, so this value can be used as an index (`as_u32() as usize`) for per-facility arrays
    /// whose length is `catalog.facilities().len()`.
    ///
    /// As with [`ItemId`], ids are catalog-scoped and must not be mixed across catalogs.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Stable identifier for a production recipe in [`Catalog`](super::Catalog).
///
/// Like [`ItemId`] and [`FacilityId`], `RecipeId` is catalog-dependent and only meaningful
/// relative to the catalog that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecipeId(u32);

impl RecipeId {
    /// Returns the underlying numeric representation.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Stable identifier for a thermal-bank power recipe in [`Catalog`](super::Catalog).
///
/// Like [`ItemId`] and [`FacilityId`], `PowerRecipeId` is catalog-dependent and only meaningful
/// relative to the catalog that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PowerRecipeId(u32);

impl PowerRecipeId {
    /// Returns the underlying numeric representation.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub(super) fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Item metadata and display texts.
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub key: Key,
    pub en: DisplayName,
    pub zh: DisplayName,
}

/// Machine facility metadata and display texts.
#[derive(Debug, Clone)]
pub struct FacilityDef {
    pub key: Key,
    pub power_w: NonZeroU32,
    pub en: DisplayName,
    pub zh: DisplayName,
}

/// Thermal bank metadata and display texts.
#[derive(Debug, Clone)]
pub struct ThermalBankDef {
    pub key: Key,
    pub en: DisplayName,
    pub zh: DisplayName,
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
