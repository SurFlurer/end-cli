use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base/core generation capacity (watts) used by the default CLI flow.
pub const P_CORE_W: u32 = 200;
/// Default external power consumption (watts) used by generated example inputs.
pub const DEFAULT_EXTERNAL_POWER_CONSUMPTION_W: u32 = 300;

/// Stable identifier for an item in [`Catalog::items`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemId(pub u32);

/// Stable identifier for a facility in [`Catalog::facilities`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FacilityId(pub u32);

/// Facility category used by the optimizer and report layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacilityKind {
    Machine,
    ThermalBank,
}

/// Item metadata and display texts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub key: String,
    pub en: String,
    pub zh: String,
}

/// Facility metadata and display texts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilityDef {
    pub key: String,
    pub kind: FacilityKind,
    pub power_w: Option<u32>,
    pub en: String,
    pub zh: String,
}

/// `(item, count)` pair used in recipes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stack {
    pub item: ItemId,
    pub count: u32,
}

/// Production recipe definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub facility: FacilityId,
    pub time_s: u32,
    pub ingredients: Vec<Stack>,
    pub products: Vec<Stack>,
}

/// Thermal-bank power recipe definition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerRecipe {
    pub ingredient: Stack,
    pub power_w: u32,
    pub time_s: u32,
}

/// Canonical in-memory model resolved from TOML inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub items: Vec<ItemDef>,
    pub facilities: Vec<FacilityDef>,
    pub recipes: Vec<Recipe>,
    pub power_recipes: Vec<PowerRecipe>,
    pub item_index: HashMap<String, ItemId>,
    pub facility_index: HashMap<String, FacilityId>,
    pub thermal_bank: FacilityId,
}

impl Catalog {
    /// Returns item metadata by id.
    pub fn item(&self, id: ItemId) -> Option<&ItemDef> {
        self.items.get(id.0 as usize)
    }

    /// Returns facility metadata by id.
    pub fn facility(&self, id: FacilityId) -> Option<&FacilityDef> {
        self.facilities.get(id.0 as usize)
    }
}

/// One outpost demand/cap configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutpostInput {
    pub key: String,
    pub en: Option<String>,
    pub zh: Option<String>,
    pub money_cap_per_hour: u32,
    pub prices: HashMap<ItemId, u32>,
}

/// Full scenario inputs consumed by optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AicInputs {
    pub external_power_consumption_w: u32,
    pub supply_per_min: HashMap<ItemId, u32>,
    pub outposts: Vec<OutpostInput>,
}
