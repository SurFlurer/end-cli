use end_model::{DisplayName, Key, ScenarioRegion};
use serde::Deserialize;
use serde::de::Error as _;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use toml::Spanned;

/// Parsed shape of `items.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemsToml {
    pub(crate) items: Box<[Spanned<ItemToml>]>,
}

/// One item entry from `items.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) key: Key,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) en: DisplayName,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) zh: DisplayName,
}

/// Parsed shape of `facilities.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FacilitiesToml {
    #[serde(default)]
    pub(crate) machines: Box<[Spanned<MachineToml>]>,
    pub(crate) thermal_bank: Spanned<ThermalBankToml>,
}

/// One machine facility entry from `facilities.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MachineToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) key: Key,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub(crate) power_w: NonZeroU32,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) en: DisplayName,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) zh: DisplayName,
    #[serde(default)]
    pub(crate) regions: Box<[ScenarioRegionToml]>,
}

/// Thermal bank facility entry from `facilities.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThermalBankToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) key: Key,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) en: DisplayName,
    #[serde(deserialize_with = "deserialize_display_name")]
    pub(crate) zh: DisplayName,
}

/// Parsed shape of `recipes.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipesToml {
    pub(crate) recipes: Box<[Spanned<RecipeToml>]>,
    #[serde(default)]
    pub(crate) power_recipes: Box<[Spanned<PowerRecipeToml>]>,
}

/// Raw `{ item, count }` pair shared by recipe and AIC inputs.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StackToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) item: Key,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub(crate) count: NonZeroU32,
}

/// One machine recipe entry from `recipes.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) facility: Key,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub(crate) time_s: NonZeroU32,
    #[serde(default = "default_empty_spanned_stack_list")]
    pub(crate) ingredients: Spanned<Box<[Spanned<StackToml>]>>,
    pub(crate) products: Spanned<Box<[Spanned<StackToml>]>>,
}

/// One power recipe entry from `recipes.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PowerRecipeToml {
    pub(crate) ingredient: Spanned<StackToml>,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub(crate) power_w: NonZeroU32,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub(crate) time_s: NonZeroU32,
}

/// Parsed shape of `aic.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AicToml {
    #[serde(
        default = "default_scenario_region",
        deserialize_with = "deserialize_scenario_region"
    )]
    pub(crate) region: ScenarioRegion,
    #[serde(deserialize_with = "deserialize_non_negative_u32")]
    pub(crate) external_power_consumption_w: u32,
    #[serde(default = "default_empty_spanned_item_positive_u32_map")]
    pub(crate) supply_per_min: Spanned<BTreeMap<KeyToml, PositiveU32Toml>>,
    #[serde(default = "default_empty_spanned_item_positive_u32_map")]
    pub(crate) external_consumption_per_min: Spanned<BTreeMap<KeyToml, PositiveU32Toml>>,
    #[serde(default)]
    pub(crate) outposts: Box<[Spanned<OutpostToml>]>,
}

/// One outpost entry from `aic.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutpostToml {
    #[serde(deserialize_with = "deserialize_key")]
    pub(crate) key: Key,
    #[serde(deserialize_with = "deserialize_non_negative_u32")]
    pub(crate) money_cap_per_hour: u32,
    #[serde(default, deserialize_with = "deserialize_optional_display_name")]
    pub(crate) en: Option<DisplayName>,
    #[serde(default, deserialize_with = "deserialize_optional_display_name")]
    pub(crate) zh: Option<DisplayName>,
    pub(crate) prices: Spanned<BTreeMap<KeyToml, NonNegativeU32Toml>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScenarioRegionToml(ScenarioRegion);

impl ScenarioRegionToml {
    pub(crate) fn into_inner(self) -> ScenarioRegion {
        self.0
    }
}

impl<'de> Deserialize<'de> for ScenarioRegionToml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_scenario_region(raw.as_str())
            .map(Self)
            .map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KeyToml(Key);

impl KeyToml {
    pub(crate) fn into_inner(self) -> Key {
        self.0
    }
}

impl<'de> Deserialize<'de> for KeyToml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Key::try_from(raw).map(Self).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PositiveU32Toml(NonZeroU32);

impl PositiveU32Toml {
    pub(crate) fn into_inner(self) -> NonZeroU32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PositiveU32Toml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        parse_positive_u32(value)
            .map(Self)
            .map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NonNegativeU32Toml(u32);

impl NonNegativeU32Toml {
    pub(crate) fn into_inner(self) -> u32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for NonNegativeU32Toml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        parse_non_negative_u32(value)
            .map(Self)
            .map_err(D::Error::custom)
    }
}

fn deserialize_key<'de, D>(deserializer: D) -> Result<Key, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    Key::try_from(raw).map_err(D::Error::custom)
}

fn deserialize_display_name<'de, D>(deserializer: D) -> Result<DisplayName, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    DisplayName::try_from(raw).map_err(D::Error::custom)
}

fn deserialize_positive_u32<'de, D>(deserializer: D) -> Result<NonZeroU32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    parse_positive_u32(value).map_err(D::Error::custom)
}

fn deserialize_non_negative_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    parse_non_negative_u32(value).map_err(D::Error::custom)
}

fn deserialize_optional_display_name<'de, D>(
    deserializer: D,
) -> Result<Option<DisplayName>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    raw.map(|text| DisplayName::try_from(text).map_err(D::Error::custom))
        .transpose()
}

fn parse_scenario_region(value: &str) -> Result<ScenarioRegion, String> {
    match value {
        "fourth_valley" => Ok(ScenarioRegion::FourthValley),
        "wuling" => Ok(ScenarioRegion::Wuling),
        other => Err(format!(
            "invalid region `{other}`, expected one of: fourth_valley, wuling"
        )),
    }
}

fn default_scenario_region() -> ScenarioRegion {
    ScenarioRegion::Wuling
}

fn deserialize_scenario_region<'de, D>(deserializer: D) -> Result<ScenarioRegion, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_scenario_region(raw.as_str()).map_err(D::Error::custom)
}

fn parse_positive_u32(value: i64) -> Result<NonZeroU32, String> {
    if value < 1 {
        return Err(format!("must be >= 1, got {value}"));
    }

    let parsed = u32::try_from(value).map_err(|_| format!("out of range for u32: {value}"))?;
    NonZeroU32::new(parsed).ok_or_else(|| format!("must be >= 1, got {value}"))
}

fn parse_non_negative_u32(value: i64) -> Result<u32, String> {
    if value < 0 {
        return Err(format!("must be >= 0, got {value}"));
    }

    u32::try_from(value).map_err(|_| format!("out of range for u32: {value}"))
}

fn default_empty_spanned_item_positive_u32_map() -> Spanned<BTreeMap<KeyToml, PositiveU32Toml>> {
    Spanned::new(0..0, BTreeMap::new())
}

fn default_empty_spanned_stack_list() -> Spanned<Box<[Spanned<StackToml>]>> {
    Spanned::new(0..0, Vec::<Spanned<StackToml>>::new().into_boxed_slice())
}
