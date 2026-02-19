use end_model::{
    AicInputs, Catalog, DisplayName, FacilityDef, ItemDef, Key, OutpostInput, Stack, ThermalBankDef,
};
use end_opt::run_two_stage;
use end_report::{Lang, build_report};
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

fn sample_catalog_and_inputs() -> (Catalog, AicInputs, end_opt::OptimizationResult) {
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

    let catalog = b.build().expect("build catalog");

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

    let result = run_two_stage(&catalog, &aic).expect("solve sample model");

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
    assert!(zh.contains("物流"));

    let en = build_report(Lang::En, &catalog, &aic, &result).expect("render en report");
    assert!(en.contains("Conclusion"));
    assert!(en.contains("Trading"));
    assert!(en.contains("Power"));
    assert!(en.contains("Production"));
    assert!(en.contains("Logistics"));
}
