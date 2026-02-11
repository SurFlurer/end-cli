use crate::schema::AicToml;
use crate::validate::{
    parse_key, parse_non_negative_u32, parse_optional_display_name, parse_positive_u32,
};
use crate::{Error, Result};
use end_model::{AicBuildError, AicInputs, Catalog, ItemNonZeroU32Map, ItemU32Map, OutpostInput};
use std::path::{Path, PathBuf};

const BUILTIN_AIC_TOML: &str = include_str!("aic.toml");
const BUILTIN_AIC_PATH: &str = "<builtin>/aic.toml";

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

/// Serialize [`default_aic`] as pretty TOML.
pub fn default_aic_toml(catalog: &Catalog) -> Result<String> {
    // validate first. because user can specify `init --data-dir`, we want to make sure the built-in AIC is valid against the potentially customized catalog.
    let path = PathBuf::from(BUILTIN_AIC_PATH);
    let raw: AicToml = toml::from_str(BUILTIN_AIC_TOML).map_err(|source| Error::TomlParse {
        path: path.clone(),
        source,
    })?;
    resolve_aic(path, raw, catalog)?;
    Ok(BUILTIN_AIC_TOML.to_string())
}

/// Convert parsed AIC TOML into validated domain inputs and resolve catalog references.
fn resolve_aic(path: PathBuf, raw: AicToml, catalog: &Catalog) -> Result<AicInputs> {
    // parse external power consumption
    let external_power_consumption_w = parse_non_negative_u32(
        &path,
        "external_power_consumption_w",
        None,
        raw.external_power_consumption_w,
    )?;

    // parse supply
    let mut supply_per_min = ItemNonZeroU32Map::with_capacity(raw.supply_per_min.len());
    for (item_key_raw, value_raw) in raw.supply_per_min {
        let item_key = parse_key(&path, "supply_per_min.key", None, item_key_raw)?;
        let value = parse_positive_u32(&path, "supply_per_min.value", None, value_raw)?;
        let item = catalog
            .item_id(item_key.as_str())
            .ok_or_else(|| Error::UnknownItem {
                path: path.clone(),
                key: item_key.to_string(),
            })?;
        supply_per_min.insert(item, value);
    }

    // parse outposts
    let mut outposts = Vec::with_capacity(raw.outposts.len());
    for (i, o) in raw.outposts.into_iter().enumerate() {
        let key = parse_key(&path, "outposts.key", Some(i), o.key)?;

        let money_cap_per_hour = parse_non_negative_u32(
            &path,
            "outposts.money_cap_per_hour",
            Some(i),
            o.money_cap_per_hour,
        )?;

        let en = parse_optional_display_name(&path, "outposts.en", Some(i), o.en)?;
        let zh = parse_optional_display_name(&path, "outposts.zh", Some(i), o.zh)?;

        let mut prices = ItemU32Map::with_capacity(o.prices.len());
        for (item_key_raw, price_raw) in o.prices {
            let item_key = parse_key(&path, "outposts.prices.key", Some(i), item_key_raw)?;
            let price = parse_non_negative_u32(&path, "outposts.prices.value", Some(i), price_raw)?;
            let item = catalog
                .item_id(item_key.as_str())
                .ok_or_else(|| Error::UnknownItem {
                    path: path.clone(),
                    key: item_key.to_string(),
                })?;
            prices.insert(item, price);
        }

        outposts.push(OutpostInput {
            key,
            en,
            zh,
            money_cap_per_hour,
            prices,
        });
    }

    AicInputs::new(external_power_consumption_w, supply_per_min, outposts)
        .map_err(|source| map_aic_build_error(&path, source))
}

/// Translate model-level AIC build errors into crate-level loading errors.
fn map_aic_build_error(path: &Path, source: AicBuildError) -> Error {
    match source {
        AicBuildError::DuplicateOutpostKey { key } => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "outpost".to_string(),
            key: key.to_string(),
        },
    }
}
