use crate::i18n::{facility_zh, item_zh};
use crate::lua_loader::{load_facility_power, load_recipes_from_dir};
use crate::model::{
    ConvertOutput, FacilitiesToml, ItemToml, ItemsToml, MachineToml, PowerRecipeToml, RecipeToml,
    RecipesToml, StackToml, ThermalBankToml, V1Recipe,
};
use crate::validate::parse_positive_u32;
use crate::{Error, Result};
use std::collections::BTreeSet;
use std::path::Path;

/// Convert one v1 input directory into in-memory TOML outputs for v2.
pub fn convert_dir(input_dir: &Path) -> Result<ConvertOutput> {
    let facility_power_path = input_dir.join("facility_power.toml");
    let recipe_dir = input_dir.join("recipe");

    let facility_power = load_facility_power(&facility_power_path)?;
    let recipes = load_recipes_from_dir(&recipe_dir)?;

    let mut recipe_rows = Vec::new();
    let mut power_recipe_rows = Vec::new();
    let mut item_set = BTreeSet::new();
    let mut machine_set = BTreeSet::new();

    for recipe in recipes {
        if recipe.facility == "Thermal Bank" {
            let (ingredient, power_w) = thermal_to_power(&recipe, &recipe_dir)?;
            item_set.insert(ingredient.item.clone());
            power_recipe_rows.push(PowerRecipeToml {
                ingredient,
                power_w,
                time_s: recipe.time_s,
            });
            continue;
        }

        machine_set.insert(recipe.facility.clone());

        let mut ingredients = Vec::with_capacity(recipe.ingredients.len());
        for stack in recipe.ingredients {
            let count = parse_positive_u32(
                &recipe_dir,
                format!("recipe ingredient count for item `{}`", stack.item),
                stack.count,
            )?;
            item_set.insert(stack.item.clone());
            ingredients.push(StackToml {
                item: stack.item,
                count: count.get(),
            });
        }

        let mut products = Vec::with_capacity(recipe.products.len());
        for stack in recipe.products {
            let count = parse_positive_u32(
                &recipe_dir,
                format!("recipe product count for item `{}`", stack.item),
                stack.count,
            )?;
            item_set.insert(stack.item.clone());
            products.push(StackToml {
                item: stack.item,
                count: count.get(),
            });
        }

        recipe_rows.push(RecipeToml {
            facility: recipe.facility,
            time_s: recipe.time_s,
            ingredients,
            products,
        });
    }

    recipe_rows.sort_by(|a, b| {
        a.facility
            .cmp(&b.facility)
            .then(a.time_s.cmp(&b.time_s))
            .then(a.ingredients.len().cmp(&b.ingredients.len()))
            .then(a.products.len().cmp(&b.products.len()))
    });

    power_recipe_rows.sort_by(|a, b| {
        a.ingredient
            .item
            .cmp(&b.ingredient.item)
            .then(a.time_s.cmp(&b.time_s))
            .then(a.power_w.cmp(&b.power_w))
    });

    let mut items = Vec::with_capacity(item_set.len());
    for item_key in item_set {
        let zh = item_zh(&item_key).ok_or_else(|| Error::MissingI18n {
            kind: "item",
            key: item_key.clone(),
        })?;
        items.push(ItemToml {
            key: item_key.clone(),
            en: item_key,
            zh,
        });
    }

    let mut machines = Vec::with_capacity(machine_set.len());
    for facility_key in machine_set {
        let power_w_i64 =
            *facility_power
                .get(&facility_key)
                .ok_or_else(|| Error::MissingFacilityPower {
                    facility: facility_key.clone(),
                })?;
        let power_w = parse_positive_u32(
            &facility_power_path,
            format!("facility_power `{facility_key}`"),
            power_w_i64,
        )?;

        let zh = facility_zh(&facility_key).ok_or_else(|| Error::MissingI18n {
            kind: "facility",
            key: facility_key.clone(),
        })?;

        machines.push(MachineToml {
            key: facility_key.clone(),
            power_w: power_w.get(),
            en: facility_key,
            zh: zh.to_string(),
        });
    }

    let thermal_bank = ThermalBankToml {
        key: "Thermal Bank".to_string(),
        en: "Thermal Bank".to_string(),
        zh: facility_zh("Thermal Bank")
            .ok_or_else(|| Error::MissingI18n {
                kind: "facility",
                key: "Thermal Bank".to_string(),
            })?
            .to_string(),
    };

    let items_toml = toml::to_string_pretty(&ItemsToml { items })
        .map_err(|source| Error::TomlSerialize { source })?;
    let facilities_toml = toml::to_string_pretty(&FacilitiesToml {
        machines,
        thermal_bank,
    })
    .map_err(|source| Error::TomlSerialize { source })?;
    let recipes_toml = toml::to_string_pretty(&RecipesToml {
        recipes: recipe_rows,
        power_recipes: power_recipe_rows,
    })
    .map_err(|source| Error::TomlSerialize { source })?;

    Ok(ConvertOutput {
        items_toml,
        facilities_toml,
        recipes_toml,
    })
}

fn thermal_to_power(recipe: &V1Recipe, path: &Path) -> Result<(StackToml, u32)> {
    if recipe.ingredients.len() != 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: "thermal bank recipe must have exactly one ingredient".to_string(),
        });
    }

    if recipe.products.len() != 1 {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: "thermal bank recipe must have exactly one product".to_string(),
        });
    }

    let power_product = &recipe.products[0];
    if power_product.item != "Power" {
        return Err(Error::Schema {
            path: path.to_path_buf(),
            message: format!(
                "thermal bank recipe product must be `Power`, got `{}`",
                power_product.item
            ),
        });
    }

    let ingredient = &recipe.ingredients[0];
    let ingredient_count = parse_positive_u32(
        path,
        format!("thermal ingredient count for `{}`", ingredient.item),
        ingredient.count,
    )?;
    let power_w = parse_positive_u32(path, "thermal Power.count".to_string(), power_product.count)?;

    Ok((
        StackToml {
            item: ingredient.item.clone(),
            count: ingredient_count.get(),
        },
        power_w.get(),
    ))
}
