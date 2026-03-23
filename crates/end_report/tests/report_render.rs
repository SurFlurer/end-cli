#![allow(clippy::unwrap_used, clippy::expect_used)]

use end_model::{
    AicInputs, Catalog, DisplayName, FacilityDef, FacilityRegions, ItemDef, Key, OutpostInput,
    PosF64, PowerConfig, Stack, ThermalBankDef,
};
use end_opt::run_two_stage;
use end_report::{Lang, build_report};
use generativity::Guard;
use generativity::make_guard;
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

fn sample_catalog_and_inputs<'cid, 'sid, 'rid>(
    guard: Guard<'cid>,
    aic_guard: Guard<'sid>,
    result_guard: Guard<'rid>,
) -> (
    Catalog<'cid>,
    AicInputs<'cid, 'sid>,
    end_model::OptimizationResult<'cid, 'sid, 'rid>,
) {
    let mut b = Catalog::builder(guard);
    let ore = b
        .add_item(ItemDef {
            key: key("Ore"),
            en: name("Ore"),
            zh: name("Ore_zh"),
            is_fluid: false,
        })
        .expect("add ore");
    let ingot = b
        .add_item(ItemDef {
            key: key("Ingot"),
            en: name("Ingot"),
            zh: name("Ingot_zh"),
            is_fluid: false,
        })
        .expect("add ingot");

    let machine = b
        .add_facility(FacilityDef {
            key: key("Smelter"),
            power_w: nz(10),
            en: name("Smelter"),
            zh: name("Smelter_zh"),
            regions: FacilityRegions::All,
        })
        .expect("add machine");
    let mut b = b
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal_Bank_zh"),
        })
        .expect("add thermal bank");

    b.push_recipe(
        machine,
        nz(60),
        vec![Stack {
            item: ore,
            count: nz(1),
        }]
        .into(),
        vec![Stack {
            item: ingot,
            count: nz(1),
        }]
        .into(),
    )
    .expect("push recipe");

    let catalog = b.build();

    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        vec![(ore, PosF64::new(1.0).expect("positive"))].into(),
    );
    aic_builder
        .add_outpost(OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 600,
            prices: vec![(ingot, 5)].into(),
        })
        .expect("valid aic outpost");
    let aic = aic_builder.build();

    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve sample model");

    (catalog, aic, result)
}

#[test]
fn build_report_contains_key_sections_in_both_languages() {
    make_guard!(guard);
    make_guard!(aic_guard);
    make_guard!(result_guard);
    let (catalog, aic, result) = sample_catalog_and_inputs(guard, aic_guard, result_guard);

    let zh = build_report(Lang::Zh, &catalog, &aic, &result).expect("render zh report");
    assert!(zh.contains("结论"));
    assert!(zh.contains("交易"));
    assert!(zh.contains("电力"));
    assert!(zh.contains("产线"));
    assert!(zh.contains("物流"));
    assert!(zh.contains("外部消耗"));

    let en = build_report(Lang::En, &catalog, &aic, &result).expect("render en report");
    assert!(en.contains("Conclusion"));
    assert!(en.contains("Trading"));
    assert!(en.contains("Power"));
    assert!(en.contains("Production"));
    assert!(en.contains("Logistics"));
    assert!(en.contains("External consumption"));
}
