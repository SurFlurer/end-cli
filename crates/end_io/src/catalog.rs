use crate::error::{
    RecipeSpanContext, map_item_build_error, map_machine_build_error, map_power_recipe_build_error,
    map_recipe_build_error, map_thermal_facility_build_error,
};
use crate::schema::{
    BuiltinCatalogToml, FacilitiesToml, ItemsToml, RecipesToml, StackToml,
};
use crate::{Error, Result};
use end_model::{
    Catalog, FacilityConsumption, FacilityDef, FacilityRegions, ItemDef, ItemId, PowerRecipe,
    Region, Stack, ThermalBankDef,
};
use generativity::Guard;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml::Spanned;

const BUILTIN_CATALOG: &str = concat!(
    include_str!("new-data/factory_items.toml"),
    include_str!("new-data/factory_machines.toml"),
    include_str!("new-data/liquid_undirectional.toml"),
    include_str!("new-data/factory_recipes.toml"),
    include_str!("new-data/battery.toml"),
    include_str!("new-data/factory_transmuters.toml"),
);

/// Load one legacy-format catalog file from an explicit `data_dir`.
struct LoadedToml<T> {
    path: PathBuf,
    src: Arc<str>,
    doc: T,
}

fn load_data_file<T: DeserializeOwned>(data_dir: &Path, filename: &str) -> Result<LoadedToml<T>> {
    let path = data_dir.join(filename);
    let src: Arc<str> = match std::fs::read_to_string(&path) {
        Ok(src) => src.into(),
        Err(source) => return Err(Error::Io { path, source }),
    };
    let doc = match toml::from_str(src.as_ref()) {
        Ok(doc) => doc,
        Err(source) => return Err(Error::TomlParse { path, source }),
    };
    Ok(LoadedToml { path, src, doc })
}

/// Load and validate catalog inputs.
///
/// When `data_dir` is `None`, the catalog fragments under `new-data` embedded at compile
/// time are used. An explicit directory retains support for the legacy `items.toml`,
/// `facilities.toml`, and `recipes.toml` layout.
pub fn load_catalog<'id>(data_dir: Option<&Path>, guard: Guard<'id>) -> Result<Catalog<'id>> {
    let (
        items_path,
        items_src,
        items,
        fac_path,
        fac_src,
        machines,
        thermal_bank,
        recipes_path,
        recipes_src,
        recipes,
        power_recipes,
        facility_consumptions,
    ) = match data_dir {
        Some(data_dir) => {
            let items: LoadedToml<ItemsToml> = load_data_file(data_dir, "items.toml")?;
            let facilities: LoadedToml<FacilitiesToml> =
                load_data_file(data_dir, "facilities.toml")?;
            let recipes: LoadedToml<RecipesToml> = load_data_file(data_dir, "recipes.toml")?;

            (
                items.path,
                items.src,
                items.doc.items,
                facilities.path,
                facilities.src,
                facilities.doc.machines,
                facilities.doc.thermal_bank,
                recipes.path,
                recipes.src,
                recipes.doc.recipes,
                recipes.doc.power_recipes,
                Box::default(),
            )
        }
        None => {
            let path = PathBuf::from("<builtin>/new-data");
            let src: Arc<str> = Arc::from(BUILTIN_CATALOG);
            let BuiltinCatalogToml {
                items,
                machines,
                thermal_bank,
                recipes,
                power_recipes,
                facility_consumptions,
            } = toml::from_str(src.as_ref()).map_err(|source| Error::TomlParse {
                path: path.clone(),
                source,
            })?;

            (
                path.clone(),
                Arc::clone(&src),
                items,
                path.clone(),
                Arc::clone(&src),
                machines,
                thermal_bank,
                path,
                src,
                recipes,
                power_recipes,
                facility_consumptions,
            )
        }
    };

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
                // Gases have the same no-storage/no-sale behavior as liquids.
                is_fluid: raw.fluid || raw.gas,
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

    // Add fixed per-machine material consumption (for example, transmuter media).
    for (i, raw) in facility_consumptions.into_iter().enumerate() {
        let span = raw.span();
        let raw = raw.into_inner();
        let facility = builder
            .facility_id(raw.facility.as_str())
            .ok_or_else(|| Error::UnknownFacility {
                path: recipes_path.clone(),
                key: raw.facility.to_string().into_boxed_str(),
                span: Some(span.clone()),
                src: Some(Arc::clone(&recipes_src)),
            })?;
        let item = builder
            .item_id(raw.item.as_str())
            .ok_or_else(|| Error::UnknownItem {
                path: recipes_path.clone(),
                key: raw.item.to_string().into_boxed_str(),
                span: Some(span.clone()),
                src: Some(Arc::clone(&recipes_src)),
            })?;
        builder
            .push_facility_consumption(FacilityConsumption {
                facility,
                item,
                count_per_min: raw.count_per_min,
            })
            .map_err(|source| Error::Schema {
                path: recipes_path.clone(),
                field: "facility_consumptions",
                index: Some(i),
                span: Some(span),
                src: Some(Arc::clone(&recipes_src)),
                message: source.to_string().into_boxed_str(),
            })?;
    }

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

fn facility_regions_from_machine_regions(regions: &[crate::schema::RegionToml]) -> FacilityRegions {
    let mut has_fourth_valley = false;
    let mut has_wuling = false;
    for region in regions {
        match region.into_inner() {
            Region::FourthValley => has_fourth_valley = true,
            Region::Wuling => has_wuling = true,
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
