#![allow(clippy::unwrap_used, clippy::expect_used)]

use generativity::make_guard;

use crate::{
    AicBuildError, Catalog, DisplayName, ItemDef, ItemPosF64Map, ItemU32Map, Key, OutpostInput,
    PosF64, PowerConfig, ThermalBankDef,
};

fn key(value: &str) -> Key {
    value.try_into().expect("valid key")
}

fn name(value: &str) -> DisplayName {
    value.try_into().expect("valid display name")
}

fn sample_catalog<'id>(
    guard: generativity::Guard<'id>,
) -> (Catalog<'id>, crate::ItemId<'id>, crate::ItemId<'id>) {
    let mut builder = Catalog::builder(guard);
    let a = builder
        .add_item(ItemDef {
            key: key("a"),
            en: name("A"),
            zh: name("A"),
            is_fluid: false,
        })
        .expect("item a should be insertable");
    let b = builder
        .add_item(ItemDef {
            key: key("b"),
            en: name("B"),
            zh: name("B"),
            is_fluid: false,
        })
        .expect("item b should be insertable");
    let builder = builder
        .add_thermal_bank(ThermalBankDef {
            key: key("thermal-bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal Bank"),
        })
        .expect("thermal bank should be insertable");
    let catalog = builder.build();
    (catalog, a, b)
}

#[test]
fn item_u32_map_keeps_unique_keys() {
    make_guard!(guard);
    let (_, item, _) = sample_catalog(guard);
    let mut map = ItemU32Map::new();

    assert_eq!(map.insert(item, 10), None);
    assert_eq!(map.insert(item, 20), Some(10));
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(item), Some(&20));
}

#[test]
fn item_u32_map_from_vec_uses_last_value_for_duplicates() {
    make_guard!(guard);
    let (_, a, b) = sample_catalog(guard);
    let map: ItemU32Map = vec![(a, 1), (b, 3), (a, 2)].into();

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(a), Some(&2));
    assert_eq!(map.get(b), Some(&3));
}

#[test]
fn item_pos_f64_map_from_vec_uses_last_value_for_duplicates() {
    make_guard!(guard);
    let (_, a, b) = sample_catalog(guard);
    let map: ItemPosF64Map = vec![
        (a, PosF64::new(1.0).expect("positive")),
        (b, PosF64::new(3.0).expect("positive")),
        (a, PosF64::new(2.0).expect("positive")),
    ]
    .into();

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(a), Some(&PosF64::new(2.0).expect("positive")));
    assert_eq!(map.get(b), Some(&PosF64::new(3.0).expect("positive")));
}

#[test]
fn aic_parse_rejects_duplicate_outpost_keys() {
    make_guard!(catalog_guard);
    let (_, _, b) = sample_catalog(catalog_guard);
    make_guard!(aic_guard);
    let camp = key("Camp");
    let mut builder = crate::AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(b, PosF64::new(1.0).expect("positive"))].into(),
        Default::default(),
    );

    builder
        .add_outpost(OutpostInput {
            key: camp.clone(),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 1,
            prices: vec![(b, 1)].into(),
        })
        .expect("first outpost insert should succeed");
    let err = builder
        .add_outpost(OutpostInput {
            key: camp.clone(),
            en: Some(name("Camp2")),
            zh: Some(name("Camp2_zh")),
            money_cap_per_hour: 2,
            prices: vec![(b, 2)].into(),
        })
        .expect_err("duplicate outpost key should fail");

    assert!(
        matches!(
            err,
            AicBuildError::DuplicateOutpostKey { ref key } if key == &camp
        ),
        "unexpected error: {err:?}"
    );
}
