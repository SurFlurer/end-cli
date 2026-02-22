use crate::error::{
    RecipeSpanContext, map_item_build_error, map_machine_build_error, map_power_recipe_build_error,
    map_recipe_build_error, map_thermal_facility_build_error,
};
use crate::schema::{FacilitiesToml, ItemsToml, RecipesToml, StackToml};
use crate::{Error, Result};
use end_model::{
    Catalog, FacilityDef, FacilityRegions, ItemDef, ItemId, PowerRecipe, ScenarioRegion, Stack,
    ThermalBankDef,
};
use generativity::Guard;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml::Spanned;

const BUILTIN_ITEMS: &str = include_str!("items.toml");
const BUILTIN_FACILITIES: &str = include_str!("facilities.toml");
const BUILTIN_RECIPES: &str = include_str!("recipes.toml");

/// Load one data file from `data_dir`, or fall back to built-in TOML contents.
///
/// Built-ins return a synthetic `<builtin>/...` path so error messages keep file context.
struct LoadedToml<T> {
    path: PathBuf,
    src: Arc<str>,
    doc: T,
}

fn load_data_file<T: DeserializeOwned>(
    data_dir: Option<&Path>,
    filename: &str,
    builtin: &'static str,
) -> Result<LoadedToml<T>> {
    let (path, src): (PathBuf, Arc<str>) = match data_dir {
        Some(dir) => {
            let path = dir.join(filename);
            let src: Arc<str> = match std::fs::read_to_string(&path) {
                Ok(src) => src.into(),
                Err(source) => return Err(Error::Io { path, source }),
            };
            (path, src)
        }
        None => (
            PathBuf::from(format!("<builtin>/{filename}")),
            Arc::from(builtin),
        ),
    };
    let doc = match toml::from_str(src.as_ref()) {
        Ok(doc) => doc,
        Err(source) => return Err(Error::TomlParse { path, source }),
    };
    Ok(LoadedToml { path, src, doc })
}

/// Load and validate catalog inputs (`items.toml`, `facilities.toml`, `recipes.toml`).
///
/// When `data_dir` is `None`, built-in TOML data embedded at compile time is used.
pub fn load_catalog<'id>(data_dir: Option<&Path>, guard: Guard<'id>) -> Result<Catalog<'id>> {
    // bring in our data
    let LoadedToml {
        path: items_path,
        src: items_src,
        doc: items_doc,
    }: LoadedToml<ItemsToml> = load_data_file(data_dir, "items.toml", BUILTIN_ITEMS)?;
    let LoadedToml {
        path: fac_path,
        src: fac_src,
        doc: facilities_doc,
    }: LoadedToml<FacilitiesToml> =
        load_data_file(data_dir, "facilities.toml", BUILTIN_FACILITIES)?;
    let LoadedToml {
        path: recipes_path,
        src: recipes_src,
        doc: recipes_doc,
    }: LoadedToml<RecipesToml> = load_data_file(data_dir, "recipes.toml", BUILTIN_RECIPES)?;
    let items = items_doc.items;
    let FacilitiesToml {
        machines,
        thermal_bank,
    } = facilities_doc;
    let RecipesToml {
        recipes,
        power_recipes,
    } = recipes_doc;

    // create a builder
    let mut builder = Catalog::builder(guard);

    // add items
    for (i, raw) in items.into_iter().enumerate() {
        let span = raw.span();
        let raw = raw.into_inner();
        builder
            .add_item(ItemDef {
                key: raw.key,
                en: raw.en,
                zh: raw.zh,
            })
            .map_err(|source| {
                map_item_build_error(&items_path, &items_src, i, Some(span), source)
            })?;
    }

    // add machines
    for (i, machine) in machines.into_iter().enumerate() {
        let span = machine.span();
        let machine = machine.into_inner();
        builder
            .add_facility(FacilityDef {
                key: machine.key,
                power_w: machine.power_w,
                en: machine.en,
                zh: machine.zh,
                regions: facility_regions_from_machine_regions(&machine.regions),
            })
            .map_err(|source| {
                map_machine_build_error(&fac_path, &fac_src, i, Some(span), source)
            })?;
    }

    // add thermal bank
    let thermal_bank_span = thermal_bank.span();
    let thermal_bank = thermal_bank.into_inner();
    let mut builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: thermal_bank.key,
            en: thermal_bank.en,
            zh: thermal_bank.zh,
        })
        .map_err(|source| {
            map_thermal_facility_build_error(&fac_path, &fac_src, Some(thermal_bank_span), source)
        })?;

    // add recipes
    for (i, raw) in recipes.into_iter().enumerate() {
        let recipe_span = raw.span();
        let raw = raw.into_inner();
        let ingredients_span = raw.ingredients.span();
        let products_span = raw.products.span();

        let facility = match builder.facility_id(raw.facility.as_str()) {
            Some(facility) => facility,
            None => {
                return Err(Error::UnknownFacility {
                    path: recipes_path.clone(),
                    key: raw.facility.to_string().into_boxed_str(),
                    span: Some(recipe_span),
                    src: Some(Arc::clone(&recipes_src)),
                });
            }
        };

        let ingredients = resolve_stack_list(&recipes_path, &recipes_src, raw.ingredients, |k| {
            builder.item_id(k)
        })?;
        let products = resolve_stack_list(&recipes_path, &recipes_src, raw.products, |k| {
            builder.item_id(k)
        })?;

        builder
            .push_recipe(facility, raw.time_s, ingredients, products)
            .map_err(|source| {
                map_recipe_build_error(
                    &recipes_path,
                    &recipes_src,
                    i,
                    RecipeSpanContext {
                        recipe: Some(recipe_span),
                        ingredients: Some(ingredients_span),
                        products: Some(products_span),
                    },
                    source,
                )
            })?;
    }

    // add power recipes
    for (i, raw) in power_recipes.into_iter().enumerate() {
        let recipe_span = raw.span();
        let raw = raw.into_inner();
        let ingredient_span = raw.ingredient.span();
        let ingredient = resolve_stack(&recipes_path, &recipes_src, raw.ingredient, |k| {
            builder.item_id(k)
        })?;
        builder
            .push_power_recipe(PowerRecipe {
                ingredient,
                power_w: raw.power_w,
                time_s: raw.time_s,
            })
            .map_err(|source| {
                map_power_recipe_build_error(
                    &recipes_path,
                    &recipes_src,
                    i,
                    Some(recipe_span),
                    Some(ingredient_span),
                    source,
                )
            })?;
    }

    // build the catalog
    Ok(builder.build())
}

fn facility_regions_from_machine_regions(regions: &[crate::schema::ScenarioRegionToml]) -> FacilityRegions {
    let mut has_fourth_valley = false;
    let mut has_wuling = false;
    for region in regions {
        match region.into_inner() {
            ScenarioRegion::FourthValley => has_fourth_valley = true,
            ScenarioRegion::Wuling => has_wuling = true,
        }
    }

    match (has_fourth_valley, has_wuling) {
        (true, true) | (false, false) => FacilityRegions::All,
        (true, false) => FacilityRegions::FourthValleyOnly,
        (false, true) => FacilityRegions::WulingOnly,
    }
}

/// Resolve a list of already-validated stack entries against catalog item ids.
pub(crate) fn resolve_stack_list<'id>(
    path: &Path,
    src: &Arc<str>,
    raw: Spanned<Box<[Spanned<StackToml>]>>,
    resolve_item: impl Fn(&str) -> Option<ItemId<'id>>,
) -> Result<Box<[Stack<'id>]>> {
    let raw = raw.into_inner();
    let mut resolved = Vec::with_capacity(raw.len());

    for stack in raw {
        resolved.push(resolve_stack(path, src, stack, &resolve_item)?);
    }

    Ok(resolved.into_boxed_slice())
}

/// Resolve one stack entry's `item` key into an internal item id.
pub(crate) fn resolve_stack<'id>(
    path: &Path,
    src: &Arc<str>,
    raw: Spanned<StackToml>,
    resolve_item: impl Fn(&str) -> Option<ItemId<'id>>,
) -> Result<Stack<'id>> {
    let span = raw.span();
    let raw = raw.into_inner();
    let item = resolve_item(raw.item.as_str()).ok_or_else(|| Error::UnknownItem {
        path: path.to_path_buf(),
        key: raw.item.to_string().into_boxed_str(),
        span: Some(span),
        src: Some(Arc::clone(src)),
    })?;
    Ok(Stack {
        item,
        count: raw.count,
    })
}
