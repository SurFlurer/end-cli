use crate::schema::{FacilitiesToml, ItemsToml, RecipesToml};
use crate::validate::{
    load_data_file, parse_positive_u32, resolve_stack, resolve_stack_list, validate_key,
    validate_non_empty, validate_text,
};
use crate::{Error, Result};
use end_model::{Catalog, FacilityDef, FacilityKind, ItemDef, PowerRecipe, Recipe};
use std::path::Path;

const BUILTIN_ITEMS: &str = include_str!("items.toml");
const BUILTIN_FACILITIES: &str = include_str!("facilities.toml");
const BUILTIN_RECIPES: &str = include_str!("recipes.toml");

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

    let mut builder = Catalog::builder();

    for (i, raw) in items_doc.items.into_iter().enumerate() {
        let key = validate_key(&items_path, "items.key", Some(i), raw.key)?;
        validate_text(&items_path, "items.en", Some(i), raw.en.as_str())?;
        validate_text(&items_path, "items.zh", Some(i), raw.zh.as_str())?;
        if builder.item_id(key.as_str()).is_some() {
            return Err(Error::DuplicateKey {
                path: items_path.clone(),
                kind: "item".to_string(),
                key,
            });
        }
        builder
            .add_item(ItemDef {
                key,
                en: raw.en,
                zh: raw.zh,
            })
            .map_err(|source| Error::Schema {
                path: items_path.clone(),
                field: "items".to_string(),
                index: Some(i),
                message: source.to_string(),
            })?;
    }

    for (i, machine) in facilities_doc.machines.into_iter().enumerate() {
        let key = validate_key(&fac_path, "machines.key", Some(i), machine.key)?;
        let power_w = parse_positive_u32(&fac_path, "machines.power_w", Some(i), machine.power_w)?;
        validate_text(&fac_path, "machines.en", Some(i), machine.en.as_str())?;
        validate_text(&fac_path, "machines.zh", Some(i), machine.zh.as_str())?;

        if builder.facility_id(key.as_str()).is_some() {
            return Err(Error::DuplicateKey {
                path: fac_path.clone(),
                kind: "facility".to_string(),
                key,
            });
        }

        builder
            .add_facility(FacilityDef {
                key,
                kind: FacilityKind::Machine,
                power_w: Some(power_w),
                en: machine.en,
                zh: machine.zh,
            })
            .map_err(|source| Error::Schema {
                path: fac_path.clone(),
                field: "machines".to_string(),
                index: Some(i),
                message: source.to_string(),
            })?;
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

    if builder.facility_id(thermal_key.as_str()).is_some() {
        return Err(Error::DuplicateKey {
            path: fac_path.clone(),
            kind: "facility".to_string(),
            key: thermal_key,
        });
    }

    let thermal_bank = builder
        .add_facility(FacilityDef {
            key: thermal_key,
            kind: FacilityKind::ThermalBank,
            power_w: None,
            en: facilities_doc.thermal_bank.en,
            zh: facilities_doc.thermal_bank.zh,
        })
        .map_err(|source| Error::Schema {
            path: fac_path.clone(),
            field: "thermal_bank".to_string(),
            index: None,
            message: source.to_string(),
        })?;

    for (i, raw) in recipes_doc.recipes.into_iter().enumerate() {
        let facility_key = validate_key(&recipes_path, "recipes.facility", Some(i), raw.facility)?;
        let facility =
            builder
                .facility_id(facility_key.as_str())
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
            |k| builder.item_id(k),
        )?;
        let products = resolve_stack_list(
            &recipes_path,
            "recipes.products",
            Some(i),
            raw.products,
            |k| builder.item_id(k),
        )?;

        builder.push_recipe(Recipe {
            facility,
            time_s: time_s.get(),
            ingredients,
            products,
        });
    }

    for (i, raw) in recipes_doc.power_recipes.into_iter().enumerate() {
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
        builder.push_power_recipe(PowerRecipe {
            ingredient,
            power_w: power_w.get(),
            time_s: time_s.get(),
        });
    }

    builder.build().map_err(|source| Error::Schema {
        path: Path::new("<memory>/catalog").to_path_buf(),
        field: "catalog".to_string(),
        index: None,
        message: source.to_string(),
    })
}
