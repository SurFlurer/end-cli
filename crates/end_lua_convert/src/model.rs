use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct V1Stack {
    pub(crate) item: String,
    pub(crate) count: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct V1Recipe {
    pub(crate) ingredients: Vec<V1Stack>,
    pub(crate) products: Vec<V1Stack>,
    pub(crate) facility: String,
    pub(crate) time_s: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FacilityPowerToml {
    #[serde(default)]
    pub(crate) facility_power: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemsToml {
    pub(crate) items: Vec<ItemToml>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ItemToml {
    pub(crate) key: String,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct FacilitiesToml {
    pub(crate) machines: Vec<MachineToml>,
    pub(crate) thermal_bank: ThermalBankToml,
}

#[derive(Debug, Serialize)]
pub(crate) struct MachineToml {
    pub(crate) key: String,
    pub(crate) power_w: u32,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ThermalBankToml {
    pub(crate) key: String,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecipesToml {
    pub(crate) recipes: Vec<RecipeToml>,
    pub(crate) power_recipes: Vec<PowerRecipeToml>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct StackToml {
    pub(crate) item: String,
    pub(crate) count: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecipeToml {
    pub(crate) facility: String,
    pub(crate) time_s: u32,
    pub(crate) ingredients: Vec<StackToml>,
    pub(crate) products: Vec<StackToml>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PowerRecipeToml {
    pub(crate) ingredient: StackToml,
    pub(crate) power_w: u32,
    pub(crate) time_s: u32,
}

/// Serialized output files generated from one conversion run.
#[derive(Debug, Clone)]
pub struct ConvertOutput {
    pub items_toml: String,
    pub facilities_toml: String,
    pub recipes_toml: String,
}
