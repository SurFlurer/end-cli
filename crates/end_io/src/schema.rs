use end_model::{DisplayName, Key, Region};
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
    pub(crate) regions: Box<[RegionToml]>,
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
        default = "default_region",
        deserialize_with = "deserialize_region"
    )]
    pub(crate) region: Region,
    #[serde(deserialize_with = "deserialize_non_negative_u32")]
    pub(crate) external_power_consumption_w: u32,
    #[serde(default = "default_empty_spanned_item_positive_u32_map")]
    pub(crate) supply_per_min: Spanned<BTreeMap<KeyToml, PositiveU32Toml>>,
    #[serde(default = "default_empty_spanned_item_positive_u32_map")]
    pub(crate) external_consumption_per_min: Spanned<BTreeMap<KeyToml, PositiveU32Toml>>,
    #[serde(default)]
    pub(crate) outposts: Box<[Spanned<OutpostToml>]>,
    #[serde(default)]
    pub(crate) stage2: Stage2Toml,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum Stage2Toml {
    #[default]
    MinMachines,
    MaxPowerSlack,
    MaxMoneySlack,
    Weighted { alpha: f64, beta: f64, gamma: f64 },
}

impl<'de> Deserialize<'de> for Stage2Toml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawStage2Toml {
            #[serde(default)]
            objective: Stage2ObjectiveToml,
            #[serde(default, deserialize_with = "deserialize_optional_non_negative_f64")]
            alpha: Option<f64>,
            #[serde(default, deserialize_with = "deserialize_optional_non_negative_f64")]
            beta: Option<f64>,
            #[serde(default, deserialize_with = "deserialize_optional_non_negative_f64")]
            gamma: Option<f64>,
        }

        let raw = RawStage2Toml::deserialize(deserializer)?;
        let has_any_weight = raw.alpha.is_some() || raw.beta.is_some() || raw.gamma.is_some();
        match raw.objective {
            Stage2ObjectiveToml::MinMachines => {
                if has_any_weight {
                    return Err(D::Error::custom(
                        "stage2.alpha/stage2.beta/stage2.gamma are only allowed when stage2.objective = `weighted`",
                    ));
                }
                Ok(Self::MinMachines)
            }
            Stage2ObjectiveToml::MaxPowerSlack => {
                if has_any_weight {
                    return Err(D::Error::custom(
                        "stage2.alpha/stage2.beta/stage2.gamma are only allowed when stage2.objective = `weighted`",
                    ));
                }
                Ok(Self::MaxPowerSlack)
            }
            Stage2ObjectiveToml::MaxMoneySlack => {
                if has_any_weight {
                    return Err(D::Error::custom(
                        "stage2.alpha/stage2.beta/stage2.gamma are only allowed when stage2.objective = `weighted`",
                    ));
                }
                Ok(Self::MaxMoneySlack)
            }
            Stage2ObjectiveToml::Weighted => Ok(Self::Weighted {
                alpha: raw.alpha.unwrap_or_else(default_stage2_weight),
                beta: raw.beta.unwrap_or_else(default_stage2_weight),
                gamma: raw.gamma.unwrap_or_else(default_stage2_weight),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum Stage2ObjectiveToml {
    #[default]
    MinMachines,
    MaxPowerSlack,
    MaxMoneySlack,
    Weighted,
}

impl<'de> Deserialize<'de> for Stage2ObjectiveToml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_stage2_objective(raw.as_str()).map_err(D::Error::custom)
    }
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
pub(crate) struct RegionToml(Region);

impl RegionToml {
    pub(crate) fn into_inner(self) -> Region {
        self.0
    }
}

impl<'de> Deserialize<'de> for RegionToml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_region(raw.as_str())
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

fn deserialize_optional_non_negative_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<f64>::deserialize(deserializer)?;
    value.map(parse_non_negative_f64).transpose().map_err(D::Error::custom)
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

fn parse_region(value: &str) -> Result<Region, String> {
    match value {
        "fourth_valley" => Ok(Region::FourthValley),
        "wuling" => Ok(Region::Wuling),
        other => Err(format!(
            "invalid region `{other}`, expected one of: fourth_valley, wuling"
        )),
    }
}

fn default_region() -> Region {
    Region::FourthValley
}

fn deserialize_region<'de, D>(deserializer: D) -> Result<Region, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_region(raw.as_str()).map_err(D::Error::custom)
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

fn parse_non_negative_f64(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err("must be a finite number".to_string());
    }
    if value < 0.0 {
        return Err(format!("must be >= 0, got {value}"));
    }
    Ok(value)
}

fn parse_stage2_objective(value: &str) -> Result<Stage2ObjectiveToml, String> {
    match value {
        "min_machines" => Ok(Stage2ObjectiveToml::MinMachines),
        "max_power_slack" => Ok(Stage2ObjectiveToml::MaxPowerSlack),
        "max_money_slack" => Ok(Stage2ObjectiveToml::MaxMoneySlack),
        "weighted" => Ok(Stage2ObjectiveToml::Weighted),
        other => Err(format!(
            "invalid stage2.objective `{other}`, expected one of: min_machines, max_power_slack, max_money_slack, weighted"
        )),
    }
}

fn default_stage2_weight() -> f64 {
    1.0
}

fn default_empty_spanned_item_positive_u32_map() -> Spanned<BTreeMap<KeyToml, PositiveU32Toml>> {
    Spanned::new(0..0, BTreeMap::new())
}

fn default_empty_spanned_stack_list() -> Spanned<Box<[Spanned<StackToml>]>> {
    Spanned::new(0..0, Vec::<Spanned<StackToml>>::new().into_boxed_slice())
}
