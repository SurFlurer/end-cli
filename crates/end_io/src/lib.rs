use end_model::{
    AicInputs, Catalog, FacilityDef, FacilityId, FacilityKind, ItemDef, ItemId, OutpostInput,
    PowerRecipe, Recipe, Stack,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Result alias for IO and schema-loading operations.
pub type Result<T> = std::result::Result<T, Error>;

const BUILTIN_ITEMS: &str = include_str!("items.toml");
const BUILTIN_FACILITIES: &str = include_str!("facilities.toml");
const BUILTIN_RECIPES: &str = include_str!("recipes.toml");

/// Errors raised while loading/validating TOML inputs.
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize TOML: {source}")]
    TomlSerialize {
        #[source]
        source: toml::ser::Error,
    },

    #[error("schema error in {path}, field `{field}`, index={index:?}: {message}")]
    Schema {
        path: PathBuf,
        field: String,
        index: Option<usize>,
        message: String,
    },

    #[error("duplicate {kind} key `{key}` in {path}")]
    DuplicateKey {
        path: PathBuf,
        kind: String,
        key: String,
    },

    #[error("unknown item `{key}` in {path}")]
    UnknownItem { path: PathBuf, key: String },

    #[error("unknown facility `{key}` in {path}")]
    UnknownFacility { path: PathBuf, key: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemsToml {
    items: Vec<ItemToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemToml {
    key: String,
    en: String,
    zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FacilitiesToml {
    #[serde(default)]
    machines: Vec<MachineToml>,
    thermal_bank: ThermalBankToml,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MachineToml {
    key: String,
    power_w: i64,
    en: String,
    zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThermalBankToml {
    key: String,
    en: String,
    zh: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipesToml {
    recipes: Vec<RecipeToml>,
    #[serde(default)]
    power_recipes: Vec<PowerRecipeToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StackToml {
    item: String,
    count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeToml {
    facility: String,
    time_s: i64,
    ingredients: Vec<StackToml>,
    products: Vec<StackToml>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PowerRecipeToml {
    ingredient: StackToml,
    power_w: i64,
    time_s: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AicToml {
    external_power_consumption_w: i64,
    #[serde(default)]
    supply_per_min: BTreeMap<String, i64>,
    #[serde(default)]
    outposts: Vec<OutpostToml>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OutpostToml {
    key: String,
    money_cap_per_hour: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zh: Option<String>,
    prices: BTreeMap<String, i64>,
}

/// Load and validate catalog inputs (`items.toml`, `facilities.toml`, `recipes.toml`).
///
/// When `data_dir` is `None`, built-in TOML data embedded at compile time is used.
pub fn load_catalog(data_dir: Option<&Path>) -> Result<Catalog> {
    let (items_path, items_src) = load_data_file(data_dir, "items.toml", BUILTIN_ITEMS)?;
    let (fac_path, fac_src) = load_data_file(data_dir, "facilities.toml", BUILTIN_FACILITIES)?;
    let (recipes_path, recipes_src) = load_data_file(data_dir, "recipes.toml", BUILTIN_RECIPES)?;

    let items_doc: ItemsToml = toml::from_str(&items_src).map_err(|source| Error::TomlParse {
        path: items_path.clone(),
        source,
    })?;
    let facilities_doc: FacilitiesToml =
        toml::from_str(&fac_src).map_err(|source| Error::TomlParse {
            path: fac_path.clone(),
            source,
        })?;
    let recipes_doc: RecipesToml =
        toml::from_str(&recipes_src).map_err(|source| Error::TomlParse {
            path: recipes_path.clone(),
            source,
        })?;

    validate_non_empty(items_doc.items.len(), &items_path, "items", None)?;
    validate_non_empty(recipes_doc.recipes.len(), &recipes_path, "recipes", None)?;

    let mut items = Vec::with_capacity(items_doc.items.len());
    let mut item_index: HashMap<String, ItemId> = HashMap::new();
    for (i, raw) in items_doc.items.into_iter().enumerate() {
        let key = validate_key(&items_path, "items.key", Some(i), raw.key)?;
        validate_text(&items_path, "items.en", Some(i), raw.en.as_str())?;
        validate_text(&items_path, "items.zh", Some(i), raw.zh.as_str())?;
        if item_index.contains_key(&key) {
            return Err(Error::DuplicateKey {
                path: items_path.clone(),
                kind: "item".to_string(),
                key,
            });
        }
        let id = ItemId(items.len() as u32);
        item_index.insert(key.clone(), id);
        items.push(ItemDef {
            key,
            en: raw.en,
            zh: raw.zh,
        });
    }

    let mut facilities = Vec::with_capacity(facilities_doc.machines.len() + 1);
    let mut facility_index: HashMap<String, FacilityId> = HashMap::new();

    for (i, machine) in facilities_doc.machines.into_iter().enumerate() {
        let key = validate_key(&fac_path, "machines.key", Some(i), machine.key)?;
        let power_w = parse_positive_u32(&fac_path, "machines.power_w", Some(i), machine.power_w)?;
        validate_text(&fac_path, "machines.en", Some(i), machine.en.as_str())?;
        validate_text(&fac_path, "machines.zh", Some(i), machine.zh.as_str())?;

        if facility_index.contains_key(&key) {
            return Err(Error::DuplicateKey {
                path: fac_path.clone(),
                kind: "facility".to_string(),
                key,
            });
        }

        let id = FacilityId(facilities.len() as u32);
        facility_index.insert(key.clone(), id);
        facilities.push(FacilityDef {
            key,
            kind: FacilityKind::Machine,
            power_w: Some(power_w),
            en: machine.en,
            zh: machine.zh,
        });
    }

    let thermal_key = validate_key(
        &fac_path,
        "thermal_bank.key",
        None,
        facilities_doc.thermal_bank.key,
    )?;
    validate_text(
        &fac_path,
        "thermal_bank.en",
        None,
        facilities_doc.thermal_bank.en.as_str(),
    )?;
    validate_text(
        &fac_path,
        "thermal_bank.zh",
        None,
        facilities_doc.thermal_bank.zh.as_str(),
    )?;

    if facility_index.contains_key(&thermal_key) {
        return Err(Error::DuplicateKey {
            path: fac_path.clone(),
            kind: "facility".to_string(),
            key: thermal_key,
        });
    }

    let thermal_bank = FacilityId(facilities.len() as u32);
    facility_index.insert(thermal_key.clone(), thermal_bank);
    facilities.push(FacilityDef {
        key: thermal_key,
        kind: FacilityKind::ThermalBank,
        power_w: None,
        en: facilities_doc.thermal_bank.en,
        zh: facilities_doc.thermal_bank.zh,
    });

    let mut recipes = Vec::with_capacity(recipes_doc.recipes.len());
    for (i, raw) in recipes_doc.recipes.into_iter().enumerate() {
        let facility_key = validate_key(&recipes_path, "recipes.facility", Some(i), raw.facility)?;
        let facility =
            *facility_index
                .get(&facility_key)
                .ok_or_else(|| Error::UnknownFacility {
                    path: recipes_path.clone(),
                    key: facility_key.clone(),
                })?;
        if facility == thermal_bank {
            return Err(Error::Schema {
                path: recipes_path.clone(),
                field: "recipes.facility".to_string(),
                index: Some(i),
                message: "thermal_bank cannot appear in recipes; use power_recipes".to_string(),
            });
        }

        let time_s = parse_positive_u32(&recipes_path, "recipes.time_s", Some(i), raw.time_s)?;
        validate_non_empty(
            raw.ingredients.len(),
            &recipes_path,
            "recipes.ingredients",
            Some(i),
        )?;
        validate_non_empty(
            raw.products.len(),
            &recipes_path,
            "recipes.products",
            Some(i),
        )?;

        let ingredients = resolve_stack_list(
            &recipes_path,
            "recipes.ingredients",
            Some(i),
            raw.ingredients,
            &item_index,
        )?;
        let products = resolve_stack_list(
            &recipes_path,
            "recipes.products",
            Some(i),
            raw.products,
            &item_index,
        )?;

        recipes.push(Recipe {
            facility,
            time_s,
            ingredients,
            products,
        });
    }

    let mut power_recipes = Vec::with_capacity(recipes_doc.power_recipes.len());
    for (i, raw) in recipes_doc.power_recipes.into_iter().enumerate() {
        let ingredient = resolve_stack(
            &recipes_path,
            "power_recipes.ingredient",
            Some(i),
            raw.ingredient,
            &item_index,
        )?;
        let power_w =
            parse_positive_u32(&recipes_path, "power_recipes.power_w", Some(i), raw.power_w)?;
        let time_s =
            parse_positive_u32(&recipes_path, "power_recipes.time_s", Some(i), raw.time_s)?;
        power_recipes.push(PowerRecipe {
            ingredient,
            power_w,
            time_s,
        });
    }

    Ok(Catalog {
        items,
        facilities,
        recipes,
        power_recipes,
        item_index,
        facility_index,
        thermal_bank,
    })
}

/// Load `aic.toml` from disk and resolve key-based references against a catalog.
pub fn load_aic(path: &Path, catalog: &Catalog) -> Result<AicInputs> {
    let src = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let raw: AicToml = toml::from_str(&src).map_err(|source| Error::TomlParse {
        path: path.to_path_buf(),
        source,
    })?;
    resolve_aic(path.to_path_buf(), raw, catalog)
}

/// Build bundled default AIC inputs and resolve them through the same validation path.
pub fn default_aic(catalog: &Catalog) -> Result<AicInputs> {
    let mut supply_per_min = BTreeMap::new();
    supply_per_min.insert("Originium Ore".to_string(), 520);
    supply_per_min.insert("Ferrium Ore".to_string(), 160);
    supply_per_min.insert("Amethyst Ore".to_string(), 160);

    let mut outposts = Vec::new();

    let mut refugee_prices = BTreeMap::new();
    refugee_prices.insert("HC Valley Battery".to_string(), 70);
    refugee_prices.insert("SC Valley Battery".to_string(), 30);
    refugee_prices.insert("Buck Capsule (A)".to_string(), 70);
    refugee_prices.insert("Buck Capsule (B)".to_string(), 27);
    refugee_prices.insert("Buck Capsule (C)".to_string(), 10);
    refugee_prices.insert("Canned Citrome (B)".to_string(), 27);
    refugee_prices.insert("Canned Citrome (C)".to_string(), 10);
    refugee_prices.insert("Origocrust".to_string(), 1);
    refugee_prices.insert("Amethyst Part".to_string(), 1);
    refugee_prices.insert("Amethyst Bottle".to_string(), 2);
    outposts.push(OutpostToml {
        key: "Refugee Camp".to_string(),
        money_cap_per_hour: 17316,
        en: Some("Refugee Camp".to_string()),
        zh: Some("难民暂居处".to_string()),
        prices: refugee_prices,
    });

    let mut infra_prices = BTreeMap::new();
    infra_prices.insert("SC Valley Battery".to_string(), 30);
    infra_prices.insert("LC Valley Battery".to_string(), 16);
    infra_prices.insert("Buck Capsule (A)".to_string(), 70);
    infra_prices.insert("Buck Capsule (B)".to_string(), 27);
    infra_prices.insert("Buck Capsule (C)".to_string(), 10);
    infra_prices.insert("Ferrium Part".to_string(), 1);
    outposts.push(OutpostToml {
        key: "Infra Station".to_string(),
        money_cap_per_hour: 27072,
        en: Some("Infra Station".to_string()),
        zh: Some("基建前站".to_string()),
        prices: infra_prices,
    });

    let raw = AicToml {
        external_power_consumption_w: 300,
        supply_per_min,
        outposts,
    };
    resolve_aic(PathBuf::from("<default>/aic.toml"), raw, catalog)
}

/// Serialize [`default_aic`] as pretty TOML.
pub fn default_aic_toml(catalog: &Catalog) -> Result<String> {
    let raw = aic_inputs_to_toml(default_aic(catalog)?, catalog)?;
    toml::to_string_pretty(&raw).map_err(|source| Error::TomlSerialize { source })
}

fn aic_inputs_to_toml(inputs: AicInputs, catalog: &Catalog) -> Result<AicToml> {
    let mut supply_per_min = BTreeMap::new();
    for (item_id, per_min) in inputs.supply_per_min {
        let key = catalog
            .item(item_id)
            .ok_or_else(|| Error::Schema {
                path: PathBuf::from("<memory>/aic.toml"),
                field: "supply_per_min".to_string(),
                index: None,
                message: format!("unknown item id {}", item_id.0),
            })?
            .key
            .clone();
        supply_per_min.insert(key, per_min as i64);
    }

    let mut outposts = Vec::with_capacity(inputs.outposts.len());
    for o in inputs.outposts {
        let mut prices = BTreeMap::new();
        for (item_id, price) in o.prices {
            let key = catalog
                .item(item_id)
                .ok_or_else(|| Error::Schema {
                    path: PathBuf::from("<memory>/aic.toml"),
                    field: "outposts.prices".to_string(),
                    index: None,
                    message: format!("unknown item id {}", item_id.0),
                })?
                .key
                .clone();
            prices.insert(key, price as i64);
        }
        outposts.push(OutpostToml {
            key: o.key,
            money_cap_per_hour: o.money_cap_per_hour as i64,
            en: o.en,
            zh: o.zh,
            prices,
        });
    }

    Ok(AicToml {
        external_power_consumption_w: inputs.external_power_consumption_w as i64,
        supply_per_min,
        outposts,
    })
}

fn resolve_aic(path: PathBuf, raw: AicToml, catalog: &Catalog) -> Result<AicInputs> {
    let external_power_consumption_w = parse_non_negative_u32(
        &path,
        "external_power_consumption_w",
        None,
        raw.external_power_consumption_w,
    )?;

    let mut supply_per_min = HashMap::new();
    for (item_key_raw, value_raw) in raw.supply_per_min {
        let item_key = validate_key(&path, "supply_per_min.key", None, item_key_raw)?;
        let value = parse_positive_u32(&path, "supply_per_min.value", None, value_raw)?;
        let item = *catalog
            .item_index
            .get(&item_key)
            .ok_or_else(|| Error::UnknownItem {
                path: path.clone(),
                key: item_key,
            })?;
        supply_per_min.insert(item, value);
    }

    let mut outposts = Vec::with_capacity(raw.outposts.len());
    let mut outpost_keys = HashSet::new();
    for (i, o) in raw.outposts.into_iter().enumerate() {
        let key = validate_key(&path, "outposts.key", Some(i), o.key)?;
        if !outpost_keys.insert(key.clone()) {
            return Err(Error::DuplicateKey {
                path: path.clone(),
                kind: "outpost".to_string(),
                key,
            });
        }

        let money_cap_per_hour = parse_non_negative_u32(
            &path,
            "outposts.money_cap_per_hour",
            Some(i),
            o.money_cap_per_hour,
        )?;

        if let Some(en) = o.en.as_deref() {
            validate_text(&path, "outposts.en", Some(i), en)?;
        }
        if let Some(zh) = o.zh.as_deref() {
            validate_text(&path, "outposts.zh", Some(i), zh)?;
        }

        let mut prices = HashMap::new();
        for (item_key_raw, price_raw) in o.prices {
            let item_key = validate_key(&path, "outposts.prices.key", Some(i), item_key_raw)?;
            let price = parse_non_negative_u32(&path, "outposts.prices.value", Some(i), price_raw)?;
            let item = *catalog
                .item_index
                .get(&item_key)
                .ok_or_else(|| Error::UnknownItem {
                    path: path.clone(),
                    key: item_key,
                })?;
            prices.insert(item, price);
        }

        outposts.push(OutpostInput {
            key,
            en: o.en,
            zh: o.zh,
            money_cap_per_hour,
            prices,
        });
    }

    Ok(AicInputs {
        external_power_consumption_w,
        supply_per_min,
        outposts,
    })
}

fn resolve_stack_list(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: Vec<StackToml>,
    item_index: &HashMap<String, ItemId>,
) -> Result<Vec<Stack>> {
    let mut resolved = Vec::with_capacity(raw.len());
    let mut seen = HashSet::new();

    for stack in raw {
        let s = resolve_stack(path, field, index, stack, item_index)?;
        if !seen.insert(s.item) {
            return Err(Error::Schema {
                path: path.to_path_buf(),
                field: field.to_string(),
                index,
                message: "duplicate item in same list".to_string(),
            });
        }
        resolved.push(s);
    }

    Ok(resolved)
}

fn resolve_stack(
    path: &Path,
    field: &str,
    index: Option<usize>,
    raw: StackToml,
    item_index: &HashMap<String, ItemId>,
) -> Result<Stack> {
    let item_key = validate_key(path, &format!("{field}.item"), index, raw.item)?;
    let count = parse_positive_u32(path, &format!("{field}.count"), index, raw.count)?;
    let item = *item_index
        .get(&item_key)
        .ok_or_else(|| Error::UnknownItem {
            path: path.to_path_buf(),
            key: item_key,
        })?;
    Ok(Stack { item, count })
}

fn validate_non_empty(len: usize, path: &Path, field: &str, index: Option<usize>) -> Result<()> {
    if len == 0 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "must not be empty".to_string(),
        });
    }
    Ok(())
}

fn validate_text(path: &Path, field: &str, index: Option<usize>, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "must not be blank".to_string(),
        });
    }
    Ok(())
}

fn validate_key(path: &Path, field: &str, index: Option<usize>, key: String) -> Result<String> {
    if key.trim().is_empty() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "key must not be blank".to_string(),
        });
    }
    if key != key.trim() {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: "key must not have leading/trailing spaces".to_string(),
        });
    }
    Ok(key)
}

fn parse_positive_u32(path: &Path, field: &str, index: Option<usize>, value: i64) -> Result<u32> {
    if value < 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: format!("must be >= 1, got {value}"),
        });
    }
    u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: format!("out of range for u32: {value}"),
    })
}

fn parse_non_negative_u32(
    path: &Path,
    field: &str,
    index: Option<usize>,
    value: i64,
) -> Result<u32> {
    if value < 0 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            field: field.to_string(),
            index,
            message: format!("must be >= 0, got {value}"),
        });
    }
    u32::try_from(value).map_err(|_| Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index,
        message: format!("out of range for u32: {value}"),
    })
}

fn load_data_file(
    data_dir: Option<&Path>,
    filename: &str,
    builtin: &'static str,
) -> Result<(PathBuf, String)> {
    match data_dir {
        Some(dir) => {
            let path = dir.join(filename);
            let src = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            Ok((path, src))
        }
        None => Ok((
            PathBuf::from(format!("<builtin>/{filename}")),
            builtin.to_string(),
        )),
    }
}
