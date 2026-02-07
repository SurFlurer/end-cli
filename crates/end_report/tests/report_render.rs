use end_model::{
    AicInputs, Catalog, FacilityDef, FacilityId, FacilityKind, ItemDef, ItemId, OutpostInput,
    Recipe, Stack,
};
use end_opt::{SolveInputs, run_two_stage};
use end_report::{Lang, build_report};
use std::collections::HashMap;

fn sample_catalog_and_inputs() -> (Catalog, AicInputs, end_opt::OptimizationResult) {
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

    let aic = AicInputs {
        external_power_consumption_w: 0,
        supply_per_min: HashMap::from([(ore, 10)]),
        outposts: vec![OutpostInput {
            key: "Camp".to_string(),
            en: Some("Camp".to_string()),
            zh: Some("Camp_zh".to_string()),
            money_cap_per_hour: 600,
            prices: HashMap::from([(ingot, 5)]),
        }],
    };

    let result = run_two_stage(
        &catalog,
        &SolveInputs {
            p_core_w: 200,
            aic: aic.clone(),
        },
    )
    .expect("solve sample model");

    (catalog, aic, result)
}

#[test]
fn build_report_contains_key_sections_in_both_languages() {
    let (catalog, aic, result) = sample_catalog_and_inputs();

    let zh = build_report(Lang::Zh, &catalog, &aic, &result).expect("render zh report");
    assert!(zh.contains("结论"));
    assert!(zh.contains("交易"));
    assert!(zh.contains("电力"));
    assert!(zh.contains("产线"));

    let en = build_report(Lang::En, &catalog, &aic, &result).expect("render en report");
    assert!(en.contains("Conclusion"));
    assert!(en.contains("Trading"));
    assert!(en.contains("Power"));
    assert!(en.contains("Production"));
}
