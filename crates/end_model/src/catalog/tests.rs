#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::num::NonZeroU32;

use generativity::Guard;
use generativity::make_guard;

use crate::{DisplayName, Key};

use super::*;

fn key(value: &str) -> Key {
    value.try_into().expect("valid key")
}

fn name(value: &str) -> DisplayName {
    value.try_into().expect("valid display name")
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("non-zero")
}

fn sample_builder<'id>(
    guard: Guard<'id>,
) -> (
    CatalogBuilder<'id>,
    ItemId<'id>,
    ItemId<'id>,
    FacilityId<'id>,
) {
    let mut builder = Catalog::builder(guard);
    let a = builder
        .add_item(ItemDef {
            key: key("A"),
            en: name("A"),
            zh: name("A"),
            is_fluid: false,
        })
        .expect("add item A");
    let b = builder
        .add_item(ItemDef {
            key: key("B"),
            en: name("B"),
            zh: name("B"),
            is_fluid: false,
        })
        .expect("add item B");
    let machine = builder
        .add_facility(FacilityDef {
            key: key("M1"),
            power_w: nz(10),
            en: name("M1"),
            zh: name("M1"),
            regions: FacilityRegions::All,
        })
        .expect("add machine");
    (builder, a, b, machine)
}

#[test]
fn push_recipe_rejects_duplicate_items_in_same_list() {
    make_guard!(guard);
    let (builder, a, b, machine) = sample_builder(guard);
    let mut builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal Bank"),
        })
        .expect("add thermal bank");
    let err = builder
        .push_recipe(
            machine,
            nz(60),
            vec![
                Stack {
                    item: a,
                    count: nz(1),
                },
                Stack {
                    item: a,
                    count: nz(2),
                },
            ]
            .into(),
            vec![Stack {
                item: b,
                count: nz(1),
            }]
            .into(),
        )
        .expect_err("duplicate ingredients should fail");

    assert!(
        matches!(
            err,
            CatalogBuildError::DuplicateRecipeIngredientItem { item_id } if item_id == a.as_u32()
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn push_recipe_allows_empty_ingredients_with_non_empty_products() {
    make_guard!(guard);
    let (builder, _a, b, machine) = sample_builder(guard);
    let mut builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal Bank"),
        })
        .expect("add thermal bank");

    let recipe_id = builder
        .push_recipe(
            machine,
            nz(1),
            Vec::new().into_boxed_slice(),
            vec![Stack {
                item: b,
                count: nz(1),
            }]
            .into(),
        )
        .expect("source recipe should be accepted");

    let catalog = builder.build();
    let recipe = catalog.recipe(recipe_id);
    assert!(recipe.ingredients.is_empty(), "ingredients should be empty");
    assert_eq!(recipe.products.len(), 1, "products should stay non-empty");
}

#[test]
fn push_recipe_allows_empty_products() {
    make_guard!(guard);
    let (builder, a, _b, machine) = sample_builder(guard);
    let mut builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal Bank"),
        })
        .expect("add thermal bank");

    let recipe_id = builder
        .push_recipe(
            machine,
            nz(1),
            vec![Stack {
                item: a,
                count: nz(1),
            }]
            .into(),
            Vec::new().into_boxed_slice(),
        )
        .expect("sink recipe should be accepted");

    let catalog = builder.build();
    let recipe = catalog.recipe(recipe_id);
    assert_eq!(recipe.ingredients.len(), 1, "ingredients should stay non-empty");
    assert!(recipe.products.is_empty(), "products should be empty");
}

#[test]
fn add_facility_rejects_duplicate_key() {
    make_guard!(guard);
    let mut builder = Catalog::builder(guard);
    let _ = builder
        .add_facility(FacilityDef {
            key: key("M2"),
            power_w: nz(10),
            en: name("M2"),
            zh: name("M2"),
            regions: FacilityRegions::All,
        })
        .expect("first facility should be accepted");
    let err = builder
        .add_facility(FacilityDef {
            key: key("M2"),
            power_w: nz(20),
            en: name("M2-dupe"),
            zh: name("M2-dupe"),
            regions: FacilityRegions::All,
        })
        .expect_err("duplicate facility key should fail");

    assert!(
        matches!(err, CatalogBuildError::DuplicateFacilityKey(ref key) if key.as_str() == "M2"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn add_thermal_bank_rejects_duplicate_machine_key() {
    make_guard!(guard);
    let mut builder = Catalog::builder(guard);
    let _ = builder
        .add_facility(FacilityDef {
            key: key("M2"),
            power_w: nz(10),
            en: name("M2"),
            zh: name("M2"),
            regions: FacilityRegions::All,
        })
        .expect("machine should be accepted");
    let err = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("M2"),
            en: name("TB"),
            zh: name("TB"),
        })
        .expect_err("thermal bank key colliding with machine should fail");

    assert!(
        matches!(err, CatalogBuildError::DuplicateFacilityKey(ref key) if key.as_str() == "M2"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn push_power_recipe_accepts_valid_values() {
    make_guard!(guard);
    let (builder, a, _, _) = sample_builder(guard);
    let mut builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal Bank"),
        })
        .expect("add thermal bank");
    builder
        .push_power_recipe(PowerRecipe {
            ingredient: Stack {
                item: a,
                count: nz(1),
            },
            power_w: nz(1),
            time_s: nz(1),
        })
        .expect("non-zero fields are accepted");
}
