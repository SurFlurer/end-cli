#![allow(clippy::unwrap_used, clippy::expect_used)]

use end_model::{
    AicInputs, Catalog, DisplayName, FacilityDef, FacilityRegions, FacilityU32Map, ItemDef, Key,
    OutpostInput, PosF64, PowerConfig, Region, Stack, Stage2Weights, ThermalBankDef,
};
use end_opt::{Error, NEAR_INT_EPS, run_two_stage};
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

fn sample_catalog<'id>(
    guard: Guard<'id>,
    with_recipes: bool,
) -> (Catalog<'id>, end_model::ItemId<'id>, end_model::ItemId<'id>) {
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

    if with_recipes {
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
    }

    let catalog = b.build();
    (catalog, ore, ingot)
}

#[test]
fn run_two_stage_applies_region_facility_restrictions() {
    make_guard!(guard);
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

    let valley_machine = b
        .add_facility(FacilityDef {
            key: key("ValleySmelter"),
            power_w: nz(10),
            en: name("ValleySmelter"),
            zh: name("ValleySmelter_zh"),
            regions: FacilityRegions::FourthValleyOnly,
        })
        .expect("add valley machine");
    let _wuling_machine = b
        .add_facility(FacilityDef {
            key: key("WulingSmelter"),
            power_w: nz(10),
            en: name("WulingSmelter"),
            zh: name("WulingSmelter_zh"),
            regions: FacilityRegions::WulingOnly,
        })
        .expect("add wuling machine");

    let mut b = b
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal_Bank_zh"),
        })
        .expect("add thermal bank");

    let valley_recipe = b
        .push_recipe(
            valley_machine,
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
        .expect("add valley recipe");

    let catalog = b.build();

    make_guard!(aic_guard);
    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        Default::default(),
    )
    .region(Region::FourthValley);
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

    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve sample model");

    assert!(
        result
            .stage2
            .recipes_used
            .iter()
            .all(|usage| usage.recipe_index == valley_recipe),
        "only valley recipe should be used under fourth_valley region"
    );
}

fn sample_catalog_and_aic<'cid, 'sid>(
    guard: Guard<'cid>,
    aic_guard: Guard<'sid>,
    with_recipes: bool,
) -> (Catalog<'cid>, AicInputs<'cid, 'sid>) {
    let (catalog, ore, ingot) = sample_catalog(guard, with_recipes);

    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        Default::default(),
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

    (catalog, aic)
}

#[test]
fn run_two_stage_allows_empty_recipes_with_direct_external_sales() {
    make_guard!(guard);
    let (catalog, ore, _ingot) = sample_catalog(guard, false);
    make_guard!(aic_guard);
    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        Default::default(),
    );
    aic_builder
        .add_outpost(OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 600,
            prices: vec![(ore, 2)].into(),
        })
        .expect("valid aic outpost");
    let aic = aic_builder.build();

    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard)
        .expect("empty recipes with direct sales should solve");

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
fn run_two_stage_respects_facility_machines_max_constraint() {
    make_guard!(guard);
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

    let smelter = b
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
        smelter,
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

    make_guard!(aic_guard);
    let facility_machines_max: FacilityU32Map<'_> = vec![(smelter, 1)].into();
    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        Default::default(),
    )
    .facility_machines_max(facility_machines_max);
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

    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve sample model");

    assert_eq!(
        result.stage1.total_machines, 1,
        "stage1 should respect facility machine cap"
    );
    assert_eq!(
        result.stage2.total_machines, 1,
        "stage2 should respect facility machine cap"
    );
    assert!(
        (result.stage1.revenue_per_min - 5.0).abs() <= 1e-9,
        "stage1 revenue should be capped at 1 ingot/min by facility cap, got {}",
        result.stage1.revenue_per_min
    );
}

#[test]
fn max_money_slack_does_not_virtualize_fluid_surplus_into_stockpile() {
    make_guard!(guard);
    let mut b = Catalog::builder(guard);
    let ore = b
        .add_item(ItemDef {
            key: key("Ore"),
            en: name("Ore"),
            zh: name("Ore_zh"),
            is_fluid: false,
        })
        .expect("add ore");
    let coolant = b
        .add_item(ItemDef {
            key: key("Coolant"),
            en: name("Coolant"),
            zh: name("Coolant_zh"),
            is_fluid: true,
        })
        .expect("add coolant");
    let mixer = b
        .add_facility(FacilityDef {
            key: key("Mixer"),
            power_w: nz(10),
            en: name("Mixer"),
            zh: name("Mixer_zh"),
            regions: FacilityRegions::All,
        })
        .expect("add mixer");
    let mut b = b
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal_Bank_zh"),
        })
        .expect("add thermal bank");
    b.push_recipe(
        mixer,
        nz(60),
        vec![Stack {
            item: ore,
            count: nz(1),
        }]
        .into(),
        vec![Stack {
            item: coolant,
            count: nz(1),
        }]
        .into(),
    )
    .expect("push recipe");
    let catalog = b.build();

    make_guard!(aic_guard);
    let mut aic_builder = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        Default::default(),
    )
    .stage2_weights(Stage2Weights {
        min_machines: 0.0,
        max_power_slack: 0.0,
        max_money_slack: 1.0,
    });
    aic_builder
        .add_outpost(OutpostInput {
            key: key("Camp"),
            en: Some(name("Camp")),
            zh: Some(name("Camp_zh")),
            money_cap_per_hour: 300,
            prices: vec![(coolant, 1)].into(),
        })
        .expect("valid aic outpost");
    let aic = aic_builder.build();

    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve sample model");

    assert!(
        (result.stage1.revenue_per_min - 5.0).abs() <= 1e-9,
        "stage1 revenue should be capped at 5/min by outpost cap, got {}",
        result.stage1.revenue_per_min
    );
    assert!(
        result.stage2.money_slack_per_min.abs() <= 1e-9,
        "fluid items must not contribute virtual money slack, got {}",
        result.stage2.money_slack_per_min
    );
    assert!(
        result
            .stage2
            .item_stockpile
            .iter()
            .all(|row| row.item != coolant),
        "fluid items must not appear in warehouse stockpile"
    );
}

#[test]
fn stage2_respects_revenue_floor_and_basic_invariants() {
    make_guard!(guard);
    make_guard!(aic_guard);
    let (catalog, aic) = sample_catalog_and_aic(guard, aic_guard, true);
    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve sample model");

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
fn run_two_stage_rejects_infeasible_external_consumption() {
    make_guard!(guard);
    let (catalog, ore, _ingot) = sample_catalog(guard, false);
    make_guard!(aic_guard);
    let aic = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(ore, PosF64::new(10.0).expect("positive"))].into(),
        vec![(ore, PosF64::new(11.0).expect("positive"))].into(),
    )
    .build();

    make_guard!(result_guard);
    let err =
        run_two_stage(&catalog, &aic, result_guard).expect_err("infeasible scenario should fail");
    assert!(
        matches!(err, Error::Solver { .. }),
        "unexpected error: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Regression: 1 input -> 2 products recipe must consume 1 input per execution
// and produce 2 products per execution. The solver should compute throughput
// from time_s (2s => 30/min/machine), independent of the product count.
//
// Reported symptom: 180/min plant_grass_1 -> 360/min carbon_mtl (1:2 ratio,
// 2s recipe, 30/min throughput) was reported as needing 139 furnaces instead
// of 6. Baseline: copper_ore 1:1 ratio, 420/min -> 14 furnaces works fine.
// ---------------------------------------------------------------------------

fn build_furnace_catalog<'id>(
    guard: Guard<'id>,
    recipe_product_count: u32,
) -> (
    Catalog<'id>,
    end_model::ItemId<'id>,
    end_model::ItemId<'id>,
    end_model::FacilityId<'id>,
) {
    let mut b = Catalog::builder(guard);
    let input = b
        .add_item(ItemDef {
            key: key("InputOre"),
            en: name("InputOre"),
            zh: name("InputOre_zh"),
            is_fluid: false,
        })
        .expect("add input");
    let output = b
        .add_item(ItemDef {
            key: key("OutputSolid"),
            en: name("OutputSolid"),
            zh: name("OutputSolid_zh"),
            is_fluid: false,
        })
        .expect("add output");
    let furnace = b
        .add_facility(FacilityDef {
            key: key("Furnace"),
            power_w: nz(5),
            en: name("Furnace"),
            zh: name("Furnace_zh"),
            regions: FacilityRegions::All,
        })
        .expect("add furnace");
    let mut b = b
        .add_thermal_bank(ThermalBankDef {
            key: key("Thermal Bank"),
            en: name("Thermal Bank"),
            zh: name("Thermal_Bank_zh"),
        })
        .expect("add thermal bank");
    b.push_recipe(
        furnace,
        nz(2),
        vec![Stack {
            item: input,
            count: nz(1),
        }]
        .into(),
        vec![Stack {
            item: output,
            count: nz(recipe_product_count),
        }]
        .into(),
    )
    .expect("push recipe");
    (b.build(), input, output, furnace)
}

fn run_furnace_test(input_per_min: f64, output_demand: f64, recipe_product_count: u32) -> u32 {
    make_guard!(catalog_guard);
    let (catalog, input, output, furnace) =
        build_furnace_catalog(catalog_guard, recipe_product_count);

    make_guard!(aic_guard);
    let aic = AicInputs::builder(
        aic_guard,
        PowerConfig::default(),
        vec![(input, PosF64::new(input_per_min).expect("positive"))].into(),
        vec![(output, PosF64::new(output_demand).expect("positive"))].into(),
    )
    // Force stage2 to minimize machines (revenue floor = 0 means any feasible plan
    // is acceptable). Without this, default MaxRevenue would accept any feasible
    // furnace count as optimal (revenue is identically 0 with no outpost).
    .stage2_weights(Stage2Weights {
        min_machines: 1.0,
        max_power_slack: 0.0,
        max_money_slack: 0.0,
    })
    .build();

    make_guard!(result_guard);
    let result = run_two_stage(&catalog, &aic, result_guard).expect("solve");

    let count = result
        .stage2
        .machines_by_facility
        .iter()
        .find(|m| m.facility == furnace)
        .map(|m| m.machines)
        .unwrap_or(0);

    eprintln!(
        "input={input_per_min}/min demand={output_demand}/min product_count={recipe_product_count} \
         -> furnaces={count} (recipes_used={:?})",
        result.stage2.recipes_used
    );
    count
}

#[test]
fn furnace_one_to_one_throughput_matches_time_s() {
    // Baseline: copper_ore (1:1, 420/min) -> 14 furnaces. Must keep working.
    let count = run_furnace_test(420.0, 420.0, 1);
    assert_eq!(count, 14, "1:1 recipe expected 14 furnaces, got {count}");
}

#[test]
fn furnace_one_to_two_throughput_matches_time_s() {
    // Bug repro: plant_grass_1 (1:2, 180/min -> 360/min) -> expected 6 furnaces.
    let count = run_furnace_test(180.0, 360.0, 2);
    assert_eq!(count, 6, "1:2 recipe expected 6 furnaces, got {count}");
}

#[test]
fn furnace_one_to_three_throughput_matches_time_s() {
    // Even higher product ratio should still match 30/min throughput.
    let count = run_furnace_test(60.0, 180.0, 3);
    assert_eq!(count, 2, "1:3 recipe expected 2 furnaces, got {count}");
}
