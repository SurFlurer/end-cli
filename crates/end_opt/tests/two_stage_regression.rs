use end_model::{
    AicInputs, Catalog, DisplayName, FacilityDef, ItemDef, Key, OutpostInput, Stack, ThermalBankDef,
};
use end_opt::{Error as OptError, NEAR_INT_EPS, run_two_stage};
use std::num::NonZeroU32;

fn key(value: &str) -> Key {
    value.try_into().expect("valid key")
}

fn name(value: &str) -> DisplayName {
    value.try_into().expect("valid display name")
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("non-zero")
}

fn sample_catalog(with_recipes: bool) -> (Catalog, end_model::ItemId, end_model::ItemId) {
    let mut b = Catalog::builder();
    let ore = b
        .add_item(ItemDef {
            key: key("Ore"),
            en: name("Ore"),
            zh: name("Ore_zh"),
        })
        .expect("add ore");
    let ingot = b
        .add_item(ItemDef {
            key: key("Ingot"),
            en: name("Ingot"),
            zh: name("Ingot_zh"),
        })
        .expect("add ingot");

    let machine = b
        .add_facility(FacilityDef {
            key: key("Smelter"),
            power_w: nz(10),
            en: name("Smelter"),
            zh: name("Smelter_zh"),
        })
        .expect("add machine");
    b.add_thermal_bank(ThermalBankDef {
        key: key("Thermal Bank"),
        en: name("Thermal Bank"),
        zh: name("Thermal_Bank_zh"),
    })
    .expect("add thermal bank");

    if with_recipes {
        b.push_recipe(
            machine,
            60,
            vec![Stack {
                item: ore,
                count: 1,
            }],
            vec![Stack {
                item: ingot,
                count: 1,
            }],
        )
        .expect("push recipe");
    }

    let catalog = b.build().expect("build catalog");
    (catalog, ore, ingot)
}

fn sample_catalog_and_aic(with_recipes: bool) -> (Catalog, AicInputs) {
    let (catalog, ore, ingot) = sample_catalog(with_recipes);

    let aic = AicInputs::new(
        0,
        vec![(ore, nz(10))].into(),
        vec![OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 600,
            prices: vec![(ingot, 5)].into(),
        }],
    )
    .expect("valid aic inputs");

    (catalog, aic)
}

#[test]
fn run_two_stage_allows_empty_recipes_with_direct_external_sales() {
    let (catalog, ore, _ingot) = sample_catalog(false);
    let aic = AicInputs::new(
        0,
        vec![(ore, nz(10))].into(),
        vec![OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 600,
            prices: vec![(ore, 2)].into(),
        }],
    )
    .expect("valid aic inputs");

    let result =
        run_two_stage(&catalog, &aic).expect("empty recipes with direct sales should solve");

    assert!(
        (result.stage1.revenue_per_min - 10.0).abs() <= 1e-9,
        "stage1 revenue should be capped at 10/min by outpost cap, got {}",
        result.stage1.revenue_per_min
    );
    let floor = (result.stage1.revenue_per_min
        - NEAR_INT_EPS * result.stage1.revenue_per_min.max(1.0))
    .max(0.0);
    assert!(
        result.stage2.revenue_per_min + 1e-7 >= floor,
        "stage2 revenue {} is lower than floor {}",
        result.stage2.revenue_per_min,
        floor
    );
    assert_eq!(
        result.stage1.total_machines, 0,
        "stage1 should use no machines"
    );
    assert_eq!(
        result.stage2.total_machines, 0,
        "stage2 should use no machines"
    );
    assert!(
        result.stage1.recipes_used.is_empty(),
        "stage1 should report no recipe usage"
    );
    assert!(
        result.stage2.recipes_used.is_empty(),
        "stage2 should report no recipe usage"
    );
    assert!(
        !result.stage1.outpost_sales_qty.is_empty(),
        "stage1 should report non-empty direct sales qty"
    );
}

#[test]
fn stage2_respects_revenue_floor_and_basic_invariants() {
    let (catalog, aic) = sample_catalog_and_aic(true);
    let result = run_two_stage(&catalog, &aic).expect("solve sample model");

    let floor = (result.stage1.revenue_per_min
        - NEAR_INT_EPS * result.stage1.revenue_per_min.max(1.0))
    .max(0.0);
    assert!(
        result.stage2.revenue_per_min + 1e-7 >= floor,
        "stage2 revenue {} is lower than floor {}",
        result.stage2.revenue_per_min,
        floor
    );
    assert!(
        result.stage2.power_margin_w >= 0,
        "power margin must be non-negative"
    );

    assert!(
        !result.stage2.outpost_sales_qty.is_empty(),
        "stage2 should include sale quantity lines"
    );
    for sale in &result.stage2.outpost_sales_qty {
        assert!(
            sale.qty_per_min.get() > 0.0,
            "sale qty must stay strictly positive"
        );
    }

    assert!(
        result.stage2.recipes_used.len() <= 20,
        "recipes_used must be capped at 20"
    );
    for pair in result.stage2.recipes_used.windows(2) {
        assert!(
            pair[0].machines >= pair[1].machines,
            "recipes_used must be sorted descending by machines"
        );
    }
}

#[test]
fn run_two_stage_rejects_out_of_bounds_aic_item_id_at_entry() {
    let (catalog, ore, _ingot) = sample_catalog(false);

    let mut other = Catalog::builder();
    let _a = other
        .add_item(ItemDef {
            key: key("A"),
            en: name("A"),
            zh: name("A_zh"),
        })
        .expect("add item A");
    let _b = other
        .add_item(ItemDef {
            key: key("B"),
            en: name("B"),
            zh: name("B_zh"),
        })
        .expect("add item B");
    let alien_item = other
        .add_item(ItemDef {
            key: key("C"),
            en: name("C"),
            zh: name("C_zh"),
        })
        .expect("add item C");
    other
        .add_thermal_bank(ThermalBankDef {
            key: key("Other Thermal Bank"),
            en: name("Other Thermal Bank"),
            zh: name("Other_Thermal_Bank_zh"),
        })
        .expect("add thermal bank");
    let _other_catalog = other.build().expect("build other catalog");

    let aic = AicInputs::new(
        0,
        vec![(alien_item, nz(10))].into(),
        vec![OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 600,
            prices: vec![(ore, 2)].into(),
        }],
    )
    .expect("valid aic inputs");

    let err = run_two_stage(&catalog, &aic).expect_err("mismatched item id should be rejected");
    match err {
        OptError::InvalidInput { message } => {
            assert!(
                message.contains("supply_per_min"),
                "error should point at supply source, got: {message}"
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}
