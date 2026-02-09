use end_model::{
    AicInputs, Catalog, FacilityDef, FacilityKind, ItemDef, OutpostInput, Recipe, Stack,
};
use end_opt::{Error, STAGE2_REVENUE_FLOOR_REL_EPS, SolveInputs, run_two_stage};
use std::num::NonZeroU32;

fn sample_catalog(with_recipes: bool) -> (Catalog, end_model::ItemId, end_model::ItemId) {
    let mut b = Catalog::builder();
    let ore = b
        .add_item(ItemDef {
            key: "Ore".to_string(),
            en: "Ore".to_string(),
            zh: "Ore_zh".to_string(),
        })
        .expect("add ore");
    let ingot = b
        .add_item(ItemDef {
            key: "Ingot".to_string(),
            en: "Ingot".to_string(),
            zh: "Ingot_zh".to_string(),
        })
        .expect("add ingot");

    let machine = b
        .add_facility(FacilityDef {
            key: "Smelter".to_string(),
            kind: FacilityKind::Machine,
            power_w: Some(NonZeroU32::new(10).expect("non-zero")),
            en: "Smelter".to_string(),
            zh: "Smelter_zh".to_string(),
        })
        .expect("add machine");
    b.add_facility(FacilityDef {
        key: "Thermal Bank".to_string(),
        kind: FacilityKind::ThermalBank,
        power_w: None,
        en: "Thermal Bank".to_string(),
        zh: "Thermal_Bank_zh".to_string(),
    })
    .expect("add thermal bank");

    if with_recipes {
        b.push_recipe(Recipe {
            facility: machine,
            time_s: 60,
            ingredients: vec![Stack {
                item: ore,
                count: 1,
            }],
            products: vec![Stack {
                item: ingot,
                count: 1,
            }],
        });
    }

    let catalog = b.build().expect("build catalog");
    (catalog, ore, ingot)
}

fn sample_catalog_and_inputs(with_recipes: bool) -> (Catalog, SolveInputs) {
    let (catalog, ore, ingot) = sample_catalog(with_recipes);

    let inputs = SolveInputs {
        p_core_w: 200,
        aic: AicInputs {
            external_power_consumption_w: 0,
            supply_per_min: vec![(ore, 10)].into(),
            outposts: vec![OutpostInput {
                key: "Camp".to_string(),
                en: Some("Camp".to_string()),
                zh: Some("Camp_zh".to_string()),
                money_cap_per_hour: 600,
                prices: vec![(ingot, 5)].into(),
            }],
        },
    };

    (catalog, inputs)
}

#[test]
fn run_two_stage_rejects_empty_recipes() {
    let (catalog, inputs) = sample_catalog_and_inputs(false);

    let err = run_two_stage(&catalog, &inputs).expect_err("empty recipes should fail");
    assert!(
        matches!(err, Error::InvalidInput { ref message } if message.contains("must not be empty")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn stage2_respects_revenue_floor_and_basic_invariants() {
    let (catalog, inputs) = sample_catalog_and_inputs(true);
    let result = run_two_stage(&catalog, &inputs).expect("solve sample model");

    let floor = (result.stage1.revenue_per_min
        - STAGE2_REVENUE_FLOOR_REL_EPS * result.stage1.revenue_per_min.max(1.0))
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
        result.stage2.top_sales.len() <= 10,
        "top_sales must be capped at 10"
    );
    for pair in result.stage2.top_sales.windows(2) {
        assert!(
            pair[0].value_per_min + 1e-9 >= pair[1].value_per_min,
            "top_sales must be sorted descending by value"
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
