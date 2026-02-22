#![allow(clippy::unwrap_used, clippy::expect_used)]

use end_io::{Error, default_aic_toml, load_aic, load_catalog};
use end_model::{AicInputs, Catalog, ScenarioRegion};
use generativity::make_guard;
use std::fs;
use tempfile::TempDir;

fn load_aic_from_str<'cid, 'sid>(
    src: &str,
    catalog: &Catalog<'cid>,
    guard: generativity::Guard<'sid>,
) -> Result<AicInputs<'cid, 'sid>, Error> {
    let dir = TempDir::new().expect("create temp dir");
    let aic_path = dir.path().join("aic.toml");
    fs::write(&aic_path, src).expect("write aic.toml");
    load_aic(&aic_path, catalog, guard)
}

fn first_two_item_keys<'a, 'id>(catalog: &'a Catalog<'id>) -> (&'a str, &'a str) {
    assert!(
        catalog.items().len() >= 2,
        "builtin catalog must contain at least two items"
    );
    (&catalog.items()[0].key, &catalog.items()[1].key)
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
fn load_aic_rejects_unknown_supply_item() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let err = load_aic_from_str(
        r#"
external_power_consumption_w = 0

[supply_per_min]
"Unknown Item" = 1
"#,
        &catalog,
        aic_guard,
    )
    .expect_err("unknown item should fail");
    assert!(
        matches!(
            err,
            Error::UnknownItem {
                ref key,
                ref span,
                ..
            } if &**key == "Unknown Item" && span.is_some()
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn load_aic_rejects_unknown_external_consumption_item() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let err = load_aic_from_str(
        r#"
external_power_consumption_w = 0

[external_consumption_per_min]
"Unknown Item" = 1
"#,
        &catalog,
        aic_guard,
    )
    .expect_err("unknown item should fail");
    assert!(
        matches!(
            err,
            Error::UnknownItem {
                ref key,
                ref span,
                ..
            } if &**key == "Unknown Item" && span.is_some()
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn load_aic_rejects_duplicate_outpost_keys() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (_, price_item) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[[outposts]]
key = "Dup"
money_cap_per_hour = 60
prices = {{ "{price_item}" = 1 }}

[[outposts]]
key = "Dup"
money_cap_per_hour = 120
prices = {{ "{price_item}" = 2 }}
"#
    );

    let err = load_aic_from_str(&src, &catalog, aic_guard)
        .expect_err("duplicate outpost key should fail");
    assert!(
        matches!(
            err,
            Error::DuplicateKey {
                ref kind,
                ref key,
                ref span,
                ..
            } if *kind == "outpost" && &**key == "Dup" && span.is_some()
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn load_aic_rejects_zero_supply_value() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (supply_item, _) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[supply_per_min]
"{supply_item}" = 0
"#
    );

    let err = load_aic_from_str(&src, &catalog, aic_guard).expect_err("zero supply should fail");
    assert_toml_parse_with_span(&err, "aic.toml", "must be >= 1, got 0");
}

#[test]
fn load_aic_rejects_zero_external_consumption_value() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (consume_item, _) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[external_consumption_per_min]
"{consume_item}" = 0
"#
    );

    let err =
        load_aic_from_str(&src, &catalog, aic_guard).expect_err("zero consumption should fail");
    assert_toml_parse_with_span(&err, "aic.toml", "must be >= 1, got 0");
}

#[test]
fn load_aic_rejects_negative_external_power() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let src = r#"
external_power_consumption_w = -1
"#;

    let err = load_aic_from_str(src, &catalog, aic_guard)
        .expect_err("negative external power should fail");
    assert_toml_parse_with_span(&err, "aic.toml", "must be >= 0, got -1");
}

#[test]
fn load_aic_rejects_negative_outpost_price() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (_, price_item) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[[outposts]]
key = "Camp"
money_cap_per_hour = 60
prices = {{ "{price_item}" = -1 }}
"#
    );

    let err = load_aic_from_str(&src, &catalog, aic_guard).expect_err("negative price should fail");
    assert_toml_parse_with_span(&err, "aic.toml", "must be >= 0, got -1");
}

#[test]
fn load_aic_rejects_outpost_key_with_spaces() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (_, price_item) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[[outposts]]
key = " Camp "
money_cap_per_hour = 60
prices = {{ "{price_item}" = 1 }}
"#
    );

    let err =
        load_aic_from_str(&src, &catalog, aic_guard).expect_err("spaced outpost key should fail");
    assert_toml_parse_with_span(
        &err,
        "aic.toml",
        "Key must not have leading/trailing spaces",
    );
}

#[test]
fn load_aic_accepts_zero_external_power_when_other_fields_valid() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let (supply_item, price_item) = first_two_item_keys(&catalog);
    let src = format!(
        r#"
external_power_consumption_w = 0

[supply_per_min]
"{supply_item}" = 1

[external_consumption_per_min]
"{supply_item}" = 1

[[outposts]]
key = "Camp"
money_cap_per_hour = 60
prices = {{ "{price_item}" = 1 }}
"#
    );

    let aic = load_aic_from_str(&src, &catalog, aic_guard).expect("valid aic should load");
    assert_eq!(aic.external_power_consumption_w(), 0);
    assert_eq!(aic.region(), ScenarioRegion::Wuling);
    assert_eq!(aic.external_consumption_per_min().len(), 1);
    assert_eq!(aic.outposts().len(), 1);
}

#[test]
fn load_aic_rejects_invalid_region() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    make_guard!(aic_guard);
    let err = load_aic_from_str(
        r#"
region = "unknown"
external_power_consumption_w = 0
"#,
        &catalog,
        aic_guard,
    )
    .expect_err("invalid region should fail");
    assert_toml_parse_with_span(&err, "aic.toml", "invalid region `unknown`");
}

#[test]
fn default_aic_toml_roundtrip_is_loadable() {
    make_guard!(catalog_guard);
    let catalog = load_catalog(None, catalog_guard).expect("load builtin catalog");
    let src = default_aic_toml(&catalog).expect("build default aic toml");
    make_guard!(aic_guard);
    let loaded =
        load_aic_from_str(&src, &catalog, aic_guard).expect("default aic toml should load");

    assert!(
        !loaded.outposts().is_empty(),
        "default aic should have outposts"
    );
    assert!(
        !loaded.external_consumption_per_min().is_empty(),
        "default aic should have external consumption rows"
    );
}
