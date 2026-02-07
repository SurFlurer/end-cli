use crate::schema::{AicToml, OutpostToml};
use crate::validate::{parse_non_negative_u32, parse_positive_u32, validate_key, validate_text};
use crate::{Error, Result};
use end_model::{AicInputs, Catalog, OutpostInput};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

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

/// Build bundled default AIC inputs and resolve them through the same validation path.
pub fn default_aic(catalog: &Catalog) -> Result<AicInputs> {
    let mut supply_per_min = BTreeMap::new();
    supply_per_min.insert("Originium Ore".to_string(), 520);
    supply_per_min.insert("Ferrium Ore".to_string(), 160);
    supply_per_min.insert("Amethyst Ore".to_string(), 160);

    let mut outposts = Vec::new();

    let mut refugee_prices = BTreeMap::new();
    refugee_prices.insert("HC Valley Battery".to_string(), 70);
    refugee_prices.insert("SC Valley Battery".to_string(), 30);
    refugee_prices.insert("Buck Capsule (A)".to_string(), 70);
    refugee_prices.insert("Buck Capsule (B)".to_string(), 27);
    refugee_prices.insert("Buck Capsule (C)".to_string(), 10);
    refugee_prices.insert("Canned Citrome (B)".to_string(), 27);
    refugee_prices.insert("Canned Citrome (C)".to_string(), 10);
    refugee_prices.insert("Origocrust".to_string(), 1);
    refugee_prices.insert("Amethyst Part".to_string(), 1);
    refugee_prices.insert("Amethyst Bottle".to_string(), 2);
    outposts.push(OutpostToml {
        key: "Refugee Camp".to_string(),
        money_cap_per_hour: 17316,
        en: Some("Refugee Camp".to_string()),
        zh: Some("难民暂居处".to_string()),
        prices: refugee_prices,
    });

    let mut infra_prices = BTreeMap::new();
    infra_prices.insert("SC Valley Battery".to_string(), 30);
    infra_prices.insert("LC Valley Battery".to_string(), 16);
    infra_prices.insert("Buck Capsule (A)".to_string(), 70);
    infra_prices.insert("Buck Capsule (B)".to_string(), 27);
    infra_prices.insert("Buck Capsule (C)".to_string(), 10);
    infra_prices.insert("Ferrium Part".to_string(), 1);
    outposts.push(OutpostToml {
        key: "Infra Station".to_string(),
        money_cap_per_hour: 27072,
        en: Some("Infra Station".to_string()),
        zh: Some("基建前站".to_string()),
        prices: infra_prices,
    });

    let raw = AicToml {
        external_power_consumption_w: 300,
        supply_per_min,
        outposts,
    };
    resolve_aic(PathBuf::from("<default>/aic.toml"), raw, catalog)
}

/// Serialize [`default_aic`] as pretty TOML.
pub fn default_aic_toml(catalog: &Catalog) -> Result<String> {
    let raw = aic_inputs_to_toml(default_aic(catalog)?, catalog)?;
    toml::to_string_pretty(&raw).map_err(|source| Error::TomlSerialize { source })
}

fn aic_inputs_to_toml(inputs: AicInputs, catalog: &Catalog) -> Result<AicToml> {
    let mut supply_per_min = BTreeMap::new();
    for (item_id, per_min) in inputs.supply_per_min {
        let key = catalog
            .item(item_id)
            .ok_or_else(|| Error::Schema {
                path: PathBuf::from("<memory>/aic.toml"),
                field: "supply_per_min".to_string(),
                index: None,
                message: format!("unknown item id {}", item_id.0),
            })?
            .key
            .clone();
        supply_per_min.insert(key, per_min as i64);
    }

    let mut outposts = Vec::with_capacity(inputs.outposts.len());
    for o in inputs.outposts {
        let mut prices = BTreeMap::new();
        for (item_id, price) in o.prices {
            let key = catalog
                .item(item_id)
                .ok_or_else(|| Error::Schema {
                    path: PathBuf::from("<memory>/aic.toml"),
                    field: "outposts.prices".to_string(),
                    index: None,
                    message: format!("unknown item id {}", item_id.0),
                })?
                .key
                .clone();
            prices.insert(key, price as i64);
        }
        outposts.push(OutpostToml {
            key: o.key,
            money_cap_per_hour: o.money_cap_per_hour as i64,
            en: o.en,
            zh: o.zh,
            prices,
        });
    }

    Ok(AicToml {
        external_power_consumption_w: inputs.external_power_consumption_w as i64,
        supply_per_min,
        outposts,
    })
}

fn resolve_aic(path: PathBuf, raw: AicToml, catalog: &Catalog) -> Result<AicInputs> {
    let external_power_consumption_w = parse_non_negative_u32(
        &path,
        "external_power_consumption_w",
        None,
        raw.external_power_consumption_w,
    )?;

    let mut supply_per_min = HashMap::new();
    for (item_key_raw, value_raw) in raw.supply_per_min {
        let item_key = validate_key(&path, "supply_per_min.key", None, item_key_raw)?;
        let value = parse_positive_u32(&path, "supply_per_min.value", None, value_raw)?;
        let item = *catalog
            .item_index
            .get(&item_key)
            .ok_or_else(|| Error::UnknownItem {
                path: path.clone(),
                key: item_key,
            })?;
        supply_per_min.insert(item, value);
    }

    let mut outposts = Vec::with_capacity(raw.outposts.len());
    let mut outpost_keys = HashSet::new();
    for (i, o) in raw.outposts.into_iter().enumerate() {
        let key = validate_key(&path, "outposts.key", Some(i), o.key)?;
        if !outpost_keys.insert(key.clone()) {
            return Err(Error::DuplicateKey {
                path: path.clone(),
                kind: "outpost".to_string(),
                key,
            });
        }

        let money_cap_per_hour = parse_non_negative_u32(
            &path,
            "outposts.money_cap_per_hour",
            Some(i),
            o.money_cap_per_hour,
        )?;

        if let Some(en) = o.en.as_deref() {
            validate_text(&path, "outposts.en", Some(i), en)?;
        }
        if let Some(zh) = o.zh.as_deref() {
            validate_text(&path, "outposts.zh", Some(i), zh)?;
        }

        let mut prices = HashMap::new();
        for (item_key_raw, price_raw) in o.prices {
            let item_key = validate_key(&path, "outposts.prices.key", Some(i), item_key_raw)?;
            let price = parse_non_negative_u32(&path, "outposts.prices.value", Some(i), price_raw)?;
            let item = *catalog
                .item_index
                .get(&item_key)
                .ok_or_else(|| Error::UnknownItem {
                    path: path.clone(),
                    key: item_key,
                })?;
            prices.insert(item, price);
        }

        outposts.push(OutpostInput {
            key,
            en: o.en,
            zh: o.zh,
            money_cap_per_hour,
            prices,
        });
    }

    Ok(AicInputs {
        external_power_consumption_w,
        supply_per_min,
        outposts,
    })
}
