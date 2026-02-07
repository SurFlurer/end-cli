use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemsToml {
    pub(crate) items: Vec<ItemToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemToml {
    pub(crate) key: String,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FacilitiesToml {
    #[serde(default)]
    pub(crate) machines: Vec<MachineToml>,
    pub(crate) thermal_bank: ThermalBankToml,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MachineToml {
    pub(crate) key: String,
    pub(crate) power_w: i64,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThermalBankToml {
    pub(crate) key: String,
    pub(crate) en: String,
    pub(crate) zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipesToml {
    pub(crate) recipes: Vec<RecipeToml>,
    #[serde(default)]
    pub(crate) power_recipes: Vec<PowerRecipeToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StackToml {
    pub(crate) item: String,
    pub(crate) count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeToml {
    pub(crate) facility: String,
    pub(crate) time_s: i64,
    pub(crate) ingredients: Vec<StackToml>,
    pub(crate) products: Vec<StackToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PowerRecipeToml {
    pub(crate) ingredient: StackToml,
    pub(crate) power_w: i64,
    pub(crate) time_s: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AicToml {
    pub(crate) external_power_consumption_w: i64,
    #[serde(default)]
    pub(crate) supply_per_min: BTreeMap<String, i64>,
    #[serde(default)]
    pub(crate) outposts: Vec<OutpostToml>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutpostToml {
    pub(crate) key: String,
    pub(crate) money_cap_per_hour: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) zh: Option<String>,
    pub(crate) prices: BTreeMap<String, i64>,
}
