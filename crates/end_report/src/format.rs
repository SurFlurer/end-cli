use crate::{Error, Lang, Result};
use end_model::{Catalog, FacilityDef, FacilityId, ItemDef, ItemId, OutpostInput, Stack};

pub(crate) fn format_recipe_label(
    lang: Lang,
    catalog: &Catalog,
    facility_id: FacilityId,
    ingredients: &[Stack],
    products: &[Stack],
    time_s: u32,
) -> Result<String> {
    let facility = facility_display_name(lang, catalog, facility_id)?;

    let mut input_side = ingredients
        .iter()
        .map(|s| {
            Ok::<_, Error>(format!(
                "{} x{}",
                item_display_name(lang, catalog, s.item)?,
                s.count
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    input_side.sort();

    let mut output_side = products
        .iter()
        .map(|s| {
            Ok::<_, Error>(format!(
                "{} x{}",
                item_display_name(lang, catalog, s.item)?,
                s.count
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    output_side.sort();

    Ok(format!(
        "[{}] {} -> {} (t={:.3}s)",
        facility,
        input_side.join(" + "),
        output_side.join(" + "),
        time_s as f64
    ))
}

pub(crate) fn outpost_display_name(lang: Lang, outpost: &OutpostInput) -> &str {
    match lang {
        Lang::Zh => outpost.zh.as_deref().unwrap_or(outpost.key.as_str()),
        Lang::En => outpost.en.as_deref().unwrap_or(outpost.key.as_str()),
    }
}

pub(crate) fn item_display_name(lang: Lang, catalog: &Catalog, item: ItemId) -> Result<&str> {
    let item_def: &ItemDef = catalog.item(item).ok_or(Error::MissingItem(item))?;
    Ok(match lang {
        Lang::Zh => item_def.zh.as_str(),
        Lang::En => item_def.en.as_str(),
    })
}

pub(crate) fn facility_display_name(
    lang: Lang,
    catalog: &Catalog,
    facility: FacilityId,
) -> Result<&str> {
    let facility_def: &FacilityDef = catalog
        .facility(facility)
        .ok_or(Error::MissingFacility(facility))?;
    Ok(match lang {
        Lang::Zh => facility_def.zh.as_str(),
        Lang::En => facility_def.en.as_str(),
    })
}

pub(crate) fn t<'a>(lang: Lang, zh: &'a str, en: &'a str) -> &'a str {
    match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
}
