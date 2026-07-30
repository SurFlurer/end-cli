#![allow(clippy::unwrap_used, clippy::expect_used)]

use end_io::{Error, load_catalog};
use generativity::make_guard;
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
    make_guard!(guard);
    load_catalog(Some(dir.path()), guard).map(|_| ())
}

fn assert_schema_location(
    err: &Error,
    expected_path_suffix: &str,
    expected_field: &str,
    expected_index: Option<usize>,
) {
    match err {
        Error::Schema {
            path,
            field,
            index,
            span,
            message,
            ..
        } => {
            assert!(
                path.ends_with(expected_path_suffix),
                "unexpected path: {path:?}"
            );
            assert_eq!(*field, expected_field, "unexpected field");
            assert_eq!(*index, expected_index, "unexpected index");
            assert!(span.is_some(), "schema error should include byte span");
            assert!(!message.is_empty(), "schema message should not be empty");
        }
        _ => panic!("unexpected error: {err:?}"),
    }
}

fn assert_toml_parse_with_span(
    err: &Error,
    expected_path_suffix: &str,
    expected_message_fragment: &str,
) {
    match err {
        Error::TomlParse { path, source } => {
            assert!(
                path.ends_with(expected_path_suffix),
                "unexpected path: {path:?}"
            );
            assert!(
                source.span().is_some(),
                "TOML error should include byte span"
            );
            assert!(
                source.message().contains(expected_message_fragment),
                "unexpected TOML message: {}",
                source.message()
            );
        }
        _ => panic!("unexpected error: {err:?}"),
    }
}

#[test]
fn load_builtin_catalog_success() {
    make_guard!(guard);
    let catalog = load_catalog(None, guard).expect("load builtin catalog");
    assert!(!catalog.items().is_empty(), "builtin catalog has no items");
    assert!(
        !catalog.recipes().is_empty(),
        "builtin catalog has no recipes"
    );

    let thermal_bank = catalog.thermal_bank();
    assert_eq!(thermal_bank.key.as_str(), "power_station_1");
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
            } if *kind == "item" && &**key == "A"
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
            Error::UnknownFacility { ref key, .. } if &**key == "Thermal Bank"
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

    assert_schema_location(&err, "recipes.toml", "recipes.ingredients", Some(0));
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
            Error::UnknownFacility { ref key, .. } if &**key == "Unknown Facility"
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
        matches!(err, Error::UnknownItem { ref key, .. } if &**key == "NoSuchItem"),
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

    assert_toml_parse_with_span(
        &err,
        "facilities.toml",
        "Key must not have leading/trailing spaces",
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

    assert_toml_parse_with_span(&err, "items.toml", "must not be blank");
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

    assert_toml_parse_with_span(&err, "facilities.toml", "must be >= 1, got 0");
}
