use end_model::{
    AicInputs, Catalog, FacilityDef, FacilityKind, ItemDef, OutpostInput, Recipe, Stack,
};
use end_opt::{SolveInputs, run_two_stage};
use end_report::{Lang, build_report};
use std::num::NonZeroU32;

fn sample_catalog_and_inputs() -> (Catalog, AicInputs, end_opt::OptimizationResult) {
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

    let catalog = b.build().expect("build catalog");

    let aic = AicInputs {
        external_power_consumption_w: 0,
        supply_per_min: vec![(ore, 10)].into(),
        outposts: vec![OutpostInput {
            key: "Camp".to_string(),
            en: Some("Camp".to_string()),
            zh: Some("Camp_zh".to_string()),
            money_cap_per_hour: 600,
            prices: vec![(ingot, 5)].into(),
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
