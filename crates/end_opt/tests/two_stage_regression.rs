use end_model::{
    AicInputs, Catalog, FacilityDef, FacilityId, FacilityKind, ItemDef, ItemId, OutpostInput,
    Recipe, Stack,
};
use end_opt::{Error, STAGE2_REVENUE_FLOOR_REL_EPS, SolveInputs, run_two_stage};
use std::collections::HashMap;

fn sample_catalog_and_inputs() -> (Catalog, SolveInputs) {
    let ore = ItemId(0);
    let ingot = ItemId(1);
    let machine = FacilityId(0);
    let thermal_bank = FacilityId(1);

    let catalog = Catalog {
        items: vec![
            ItemDef {
                key: "Ore".to_string(),
                en: "Ore".to_string(),
                zh: "Ore_zh".to_string(),
            },
            ItemDef {
                key: "Ingot".to_string(),
                en: "Ingot".to_string(),
                zh: "Ingot_zh".to_string(),
            },
        ],
        facilities: vec![
            FacilityDef {
                key: "Smelter".to_string(),
                kind: FacilityKind::Machine,
                power_w: Some(10),
                en: "Smelter".to_string(),
                zh: "Smelter_zh".to_string(),
            },
            FacilityDef {
                key: "Thermal Bank".to_string(),
                kind: FacilityKind::ThermalBank,
                power_w: None,
                en: "Thermal Bank".to_string(),
                zh: "Thermal_Bank_zh".to_string(),
            },
        ],
        recipes: vec![Recipe {
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
        }],
        power_recipes: vec![],
        item_index: HashMap::from([("Ore".to_string(), ore), ("Ingot".to_string(), ingot)]),
        facility_index: HashMap::from([
            ("Smelter".to_string(), machine),
            ("Thermal Bank".to_string(), thermal_bank),
        ]),
        thermal_bank,
    };

    let inputs = SolveInputs {
        p_core_w: 200,
        aic: AicInputs {
            external_power_consumption_w: 0,
            supply_per_min: HashMap::from([(ore, 10)]),
            outposts: vec![OutpostInput {
                key: "Camp".to_string(),
                en: Some("Camp".to_string()),
                zh: Some("Camp_zh".to_string()),
                money_cap_per_hour: 600,
                prices: HashMap::from([(ingot, 5)]),
            }],
        },
    };

    (catalog, inputs)
}

#[test]
fn run_two_stage_rejects_empty_recipes() {
    let (mut catalog, inputs) = sample_catalog_and_inputs();
    catalog.recipes.clear();

    let err = run_two_stage(&catalog, &inputs).expect_err("empty recipes should fail");
    assert!(
        matches!(err, Error::InvalidInput { ref message } if message.contains("must not be empty")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn stage2_respects_revenue_floor_and_basic_invariants() {
    let (catalog, inputs) = sample_catalog_and_inputs();
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
