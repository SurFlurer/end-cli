use end_io::{Error, load_catalog};
use end_model::FacilityKind;
use std::fs;
use tempfile::TempDir;

const VALID_ITEMS: &str = r#"
[[items]]
key = "A"
en = "A"
zh = "A_zh"

[[items]]
key = "B"
en = "B"
zh = "B_zh"
"#;

const VALID_FACILITIES: &str = r#"
[[machines]]
key = "M1"
power_w = 10
en = "M1"
zh = "M1_zh"

[thermal_bank]
key = "Thermal Bank"
en = "Thermal Bank"
zh = "Thermal_Bank_zh"
"#;

const VALID_RECIPES: &str = r#"
[[recipes]]
facility = "M1"
time_s = 1
ingredients = [{ item = "A", count = 1 }]
products = [{ item = "B", count = 1 }]
"#;

fn write_catalog_files(dir: &TempDir, items: &str, facilities: &str, recipes: &str) {
    fs::write(dir.path().join("items.toml"), items).expect("write items.toml");
    fs::write(dir.path().join("facilities.toml"), facilities).expect("write facilities.toml");
    fs::write(dir.path().join("recipes.toml"), recipes).expect("write recipes.toml");
}

fn load_catalog_from_parts(items: &str, facilities: &str, recipes: &str) -> Result<(), Error> {
    let dir = TempDir::new().expect("create temp dir");
    write_catalog_files(&dir, items, facilities, recipes);
    load_catalog(Some(dir.path())).map(|_| ())
}

#[test]
fn load_builtin_catalog_success() {
    let catalog = load_catalog(None).expect("load builtin catalog");
    assert!(!catalog.items.is_empty(), "builtin catalog has no items");
    assert!(
        !catalog.recipes.is_empty(),
        "builtin catalog has no recipes"
    );

    let thermal_bank = catalog
        .facility(catalog.thermal_bank)
        .expect("thermal bank id must resolve");
    assert_eq!(thermal_bank.kind, FacilityKind::ThermalBank);
}

#[test]
fn duplicate_item_key_returns_error() {
    let err = load_catalog_from_parts(
        r#"
[[items]]
key = "A"
en = "A"
zh = "A_zh"

[[items]]
key = "A"
en = "A_dup"
zh = "A_dup_zh"
"#,
        r#"
[[machines]]
key = "M1"
power_w = 10
en = "M1"
zh = "M1_zh"

[thermal_bank]
key = "Thermal Bank"
en = "Thermal Bank"
zh = "Thermal_Bank_zh"
"#,
        r#"
[[recipes]]
facility = "M1"
time_s = 1
ingredients = [{ item = "A", count = 1 }]
products = [{ item = "A", count = 1 }]
"#,
    )
    .expect_err("duplicate item key should fail");
    assert!(
        matches!(
            err,
            Error::DuplicateKey {
                ref kind,
                ref key,
                ..
            } if kind == "item" && key == "A"
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn recipe_cannot_use_thermal_bank_facility() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        VALID_FACILITIES,
        r#"
[[recipes]]
facility = "Thermal Bank"
time_s = 1
ingredients = [{ item = "A", count = 1 }]
products = [{ item = "B", count = 1 }]
"#,
    )
    .expect_err("thermal bank in recipes should fail");

    assert!(
        matches!(
            err,
            Error::Schema {
                ref field,
                index: Some(0),
                ref message,
                ..
            } if field == "recipes.facility" && message.contains("thermal_bank cannot appear")
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn duplicate_item_in_recipe_ingredients_returns_schema_error() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        VALID_FACILITIES,
        r#"
[[recipes]]
facility = "M1"
time_s = 1
ingredients = [{ item = "A", count = 1 }, { item = "A", count = 2 }]
products = [{ item = "B", count = 1 }]
"#,
    )
    .expect_err("duplicate stack items should fail");

    assert!(
        matches!(
            err,
            Error::Schema {
                ref field,
                index: Some(0),
                ref message,
                ..
            } if field == "recipes.ingredients" && message.contains("duplicate item")
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn unknown_recipe_facility_returns_unknown_facility_error() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        VALID_FACILITIES,
        r#"
[[recipes]]
facility = "Unknown Facility"
time_s = 1
ingredients = [{ item = "A", count = 1 }]
products = [{ item = "B", count = 1 }]
"#,
    )
    .expect_err("unknown facility should fail");

    assert!(
        matches!(
            err,
            Error::UnknownFacility { ref key, .. } if key == "Unknown Facility"
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn unknown_item_in_recipe_returns_unknown_item_error() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        VALID_FACILITIES,
        r#"
[[recipes]]
facility = "M1"
time_s = 1
ingredients = [{ item = "NoSuchItem", count = 1 }]
products = [{ item = "B", count = 1 }]
"#,
    )
    .expect_err("unknown item in recipe should fail");

    assert!(
        matches!(err, Error::UnknownItem { ref key, .. } if key == "NoSuchItem"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn key_with_leading_or_trailing_spaces_is_rejected() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        r#"
[[machines]]
key = " M1 "
power_w = 10
en = "M1"
zh = "M1_zh"

[thermal_bank]
key = "Thermal Bank"
en = "Thermal Bank"
zh = "Thermal_Bank_zh"
"#,
        VALID_RECIPES,
    )
    .expect_err("spaced key should fail");

    assert!(
        matches!(
            err,
            Error::Schema {
                ref field,
                ref message,
                ..
            } if field == "machines.key" && message.contains("leading/trailing")
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn blank_i18n_text_is_rejected() {
    let err = load_catalog_from_parts(
        r#"
[[items]]
key = "A"
en = "A"
zh = "   "

[[items]]
key = "B"
en = "B"
zh = "B_zh"
"#,
        VALID_FACILITIES,
        VALID_RECIPES,
    )
    .expect_err("blank zh text should fail");

    assert!(
        matches!(
            err,
            Error::Schema {
                ref field,
                ref message,
                ..
            } if field == "items.zh" && message.contains("must not be blank")
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn non_positive_machine_power_is_rejected() {
    let err = load_catalog_from_parts(
        VALID_ITEMS,
        r#"
[[machines]]
key = "M1"
power_w = 0
en = "M1"
zh = "M1_zh"

[thermal_bank]
key = "Thermal Bank"
en = "Thermal Bank"
zh = "Thermal_Bank_zh"
"#,
        VALID_RECIPES,
    )
    .expect_err("machine power must be >= 1");

    assert!(
        matches!(
            err,
            Error::Schema {
                ref field,
                ref message,
                ..
            } if field == "machines.power_w" && message.contains("must be >= 1")
        ),
        "unexpected error: {err:?}"
    );
}
