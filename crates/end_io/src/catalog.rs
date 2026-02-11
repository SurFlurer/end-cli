use crate::schema::{FacilitiesToml, ItemsToml, RecipesToml};
use crate::validate::{
    parse_display_name, parse_key, parse_positive_u32, resolve_stack, resolve_stack_list,
};
use crate::{Error, Result};
use end_model::{Catalog, CatalogBuildError, FacilityDef, FacilityKind, ItemDef, PowerRecipe};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

const BUILTIN_ITEMS: &str = include_str!("items.toml");
const BUILTIN_FACILITIES: &str = include_str!("facilities.toml");
const BUILTIN_RECIPES: &str = include_str!("recipes.toml");

/// Load one data file from `data_dir`, or fall back to built-in TOML contents.
///
/// Built-ins return a synthetic `<builtin>/...` path so error messages keep file context.
fn load_data_file<T: DeserializeOwned>(
    data_dir: Option<&Path>,
    filename: &str,
    builtin: &'static str,
) -> Result<(PathBuf, T)> {
    let (path, src) = match data_dir {
        Some(dir) => {
            let path = dir.join(filename);
            let src = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            (path, src)
        }
        None => (
            PathBuf::from(format!("<builtin>/{filename}")),
            builtin.to_string(),
        ),
    };
    let doc = toml::from_str(&src).map_err(|source| Error::TomlParse {
        path: path.clone(),
        source,
    })?;
    Ok((path, doc))
}

/// Load and validate catalog inputs (`items.toml`, `facilities.toml`, `recipes.toml`).
///
/// When `data_dir` is `None`, built-in TOML data embedded at compile time is used.
pub fn load_catalog(data_dir: Option<&Path>) -> Result<Catalog> {
    // bring in our data
    let (items_path, items_doc): (_, ItemsToml) =
        load_data_file(data_dir, "items.toml", BUILTIN_ITEMS)?;
    let (fac_path, facilities_doc): (_, FacilitiesToml) =
        load_data_file(data_dir, "facilities.toml", BUILTIN_FACILITIES)?;
    let (recipes_path, recipes_doc): (_, RecipesToml) =
        load_data_file(data_dir, "recipes.toml", BUILTIN_RECIPES)?;
    let items = items_doc.items;
    let (machines, thermal_bank) = (facilities_doc.machines, facilities_doc.thermal_bank);
    let (recipes, power_recipes) = (recipes_doc.recipes, recipes_doc.power_recipes);

    // create a builder
    let mut builder = Catalog::builder();

    // add items
    for (i, raw) in items.into_iter().enumerate() {
        let key = parse_key(&items_path, "items.key", Some(i), raw.key)?;
        let en = parse_display_name(&items_path, "items.en", Some(i), raw.en)?;
        let zh = parse_display_name(&items_path, "items.zh", Some(i), raw.zh)?;
        builder
            .add_item(ItemDef { key, en, zh })
            .map_err(|source| map_item_build_error(&items_path, i, source))?;
    }

    // add machines
    for (i, machine) in machines.into_iter().enumerate() {
        let key = parse_key(&fac_path, "machines.key", Some(i), machine.key)?;
        let power_w = parse_positive_u32(&fac_path, "machines.power_w", Some(i), machine.power_w)?;
        let en = parse_display_name(&fac_path, "machines.en", Some(i), machine.en)?;
        let zh = parse_display_name(&fac_path, "machines.zh", Some(i), machine.zh)?;

        builder
            .add_facility(FacilityDef {
                key,
                kind: FacilityKind::Machine,
                power_w: Some(power_w),
                en,
                zh,
            })
            .map_err(|source| map_machine_build_error(&fac_path, i, source))?;
    }

    // add thermal bank
    let bank_key = parse_key(&fac_path, "thermal_bank.key", None, thermal_bank.key)?;
    let bank_en = parse_display_name(&fac_path, "thermal_bank.en", None, thermal_bank.en)?;
    let bank_zh = parse_display_name(&fac_path, "thermal_bank.zh", None, thermal_bank.zh)?;
    builder
        .add_facility(FacilityDef {
            key: bank_key,
            kind: FacilityKind::ThermalBank,
            power_w: None,
            en: bank_en,
            zh: bank_zh,
        })
        .map_err(|source| map_thermal_facility_build_error(&fac_path, source))?;
    
    // add recipes
    for (i, raw) in recipes.into_iter().enumerate() {
        let facility_key = parse_key(&recipes_path, "recipes.facility", Some(i), raw.facility)?;
        let facility =
            builder
                .facility_id(facility_key.as_str())
                .ok_or_else(|| Error::UnknownFacility {
                    path: recipes_path.clone(),
                    key: facility_key.to_string(),
                })?;

        let time_s = parse_positive_u32(&recipes_path, "recipes.time_s", Some(i), raw.time_s)?;

        let ingredients = resolve_stack_list(
            &recipes_path,
            "recipes.ingredients",
            Some(i),
            raw.ingredients,
            |k| builder.item_id(k),
        )?;
        let products = resolve_stack_list(
            &recipes_path,
            "recipes.products",
            Some(i),
            raw.products,
            |k| builder.item_id(k),
        )?;

        builder
            .push_recipe(facility, time_s.get(), ingredients, products)
            .map_err(|source| map_recipe_build_error(&recipes_path, i, source))?;
    }

    // add power recipes
    for (i, raw) in power_recipes.into_iter().enumerate() {
        let ingredient = resolve_stack(
            &recipes_path,
            "power_recipes.ingredient",
            Some(i),
            raw.ingredient,
            |k| builder.item_id(k),
        )?;
        let power_w =
            parse_positive_u32(&recipes_path, "power_recipes.power_w", Some(i), raw.power_w)?;
        let time_s =
            parse_positive_u32(&recipes_path, "power_recipes.time_s", Some(i), raw.time_s)?;
        builder
            .push_power_recipe(PowerRecipe {
                ingredient,
                power_w: power_w.get(),
                time_s: time_s.get(),
            })
            .map_err(|source| map_power_recipe_build_error(&recipes_path, i, source))?;
    }

    // build the catalog
    builder.build().map_err(|source| Error::Schema {
        path: Path::new("<memory>/catalog").to_path_buf(),
        field: "catalog".to_string(),
        index: None,
        message: source.to_string(),
    })
}

/// Re-map item-related builder errors to precise user-facing schema fields.
fn map_item_build_error(path: &Path, index: usize, source: CatalogBuildError) -> Error {
    match source {
        CatalogBuildError::DuplicateItemKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "item".to_string(),
            key: key.to_string(),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "items".to_string(),
            index: Some(index),
            message: source.to_string(),
        },
    }
}

/// Re-map machine-related builder errors to precise user-facing schema fields.
fn map_machine_build_error(path: &Path, index: usize, source: CatalogBuildError) -> Error {
    match source {
        CatalogBuildError::DuplicateFacilityKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "facility".to_string(),
            key: key.to_string(),
        },
        CatalogBuildError::MachineFacilityMissingPower { .. } => Error::Schema {
            path: path.to_path_buf(),
            field: "machines.power_w".to_string(),
            index: Some(index),
            message: source.to_string(),
        },
        CatalogBuildError::ThermalBankFacilityHasPower { .. } => Error::Schema {
            path: path.to_path_buf(),
            field: "machines".to_string(),
            index: Some(index),
            message: source.to_string(),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "machines".to_string(),
            index: Some(index),
            message: source.to_string(),
        },
    }
}

/// Re-map thermal-bank builder errors to the top-level `thermal_bank` section.
fn map_thermal_facility_build_error(path: &Path, source: CatalogBuildError) -> Error {
    match source {
        CatalogBuildError::DuplicateFacilityKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "facility".to_string(),
            key: key.to_string(),
        },
        CatalogBuildError::MachineFacilityMissingPower { .. }
        | CatalogBuildError::ThermalBankFacilityHasPower { .. } => Error::Schema {
            path: path.to_path_buf(),
            field: "thermal_bank".to_string(),
            index: None,
            message: source.to_string(),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "thermal_bank".to_string(),
            index: None,
            message: source.to_string(),
        },
    }
}

/// Collapse recipe build errors into the most specific TOML field path possible.
fn map_recipe_build_error(path: &Path, index: usize, source: CatalogBuildError) -> Error {
    let field = match source {
        CatalogBuildError::UnknownRecipeFacilityId(_)
        | CatalogBuildError::RecipeFacilityMustBeMachine { .. } => "recipes.facility",
        CatalogBuildError::RecipeTimeMustBePositive => "recipes.time_s",
        CatalogBuildError::RecipeIngredientsMustNotBeEmpty
        | CatalogBuildError::UnknownRecipeItemId {
            list: "ingredients",
            ..
        }
        | CatalogBuildError::DuplicateRecipeItem {
            list: "ingredients",
            ..
        }
        | CatalogBuildError::RecipeStackCountMustBePositive {
            list: "ingredients",
            ..
        } => "recipes.ingredients",
        CatalogBuildError::RecipeProductsMustNotBeEmpty
        | CatalogBuildError::UnknownRecipeItemId {
            list: "products", ..
        }
        | CatalogBuildError::DuplicateRecipeItem {
            list: "products", ..
        }
        | CatalogBuildError::RecipeStackCountMustBePositive {
            list: "products", ..
        } => "recipes.products",
        _ => "recipes",
    };
    Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index: Some(index),
        message: source.to_string(),
    }
}

/// Collapse power-recipe build errors into the most specific TOML field path possible.
fn map_power_recipe_build_error(path: &Path, index: usize, source: CatalogBuildError) -> Error {
    let field = match source {
        CatalogBuildError::UnknownPowerRecipeIngredientItemId(_)
        | CatalogBuildError::PowerRecipeIngredientCountMustBePositive { .. } => {
            "power_recipes.ingredient"
        }
        CatalogBuildError::PowerRecipePowerMustBePositive => "power_recipes.power_w",
        CatalogBuildError::PowerRecipeTimeMustBePositive => "power_recipes.time_s",
        _ => "power_recipes",
    };
    Error::Schema {
        path: path.to_path_buf(),
        field: field.to_string(),
        index: Some(index),
        message: source.to_string(),
    }
}
