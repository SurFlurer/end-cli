use aic_two_brand_lab::{AicInputs, Catalog, build_report, run_two_stage, use_item_with_aic};
use generativity::make_guard;

fn run_one_scenario<'cid, 'sid>(
    catalog: &Catalog<'cid>,
    item: aic_two_brand_lab::ItemId<'cid>,
    aic: &AicInputs<'cid, 'sid>,
) {
    use_item_with_aic(catalog, aic, item);
    make_guard!(result_guard);
    let solved = run_two_stage(catalog, aic, result_guard).expect("scenario should solve");
    let report = build_report(catalog, aic, &solved).expect("report should render");
    let _ = report;
}

fn main() {
    make_guard!(catalog_guard);
    let catalog = Catalog::build(catalog_guard);
    let item = catalog.item_id("ore").expect("known item");

    // Two scenarios share one catalog: item ids are reusable.
    make_guard!(scenario_guard_1);
    let aic1 = AicInputs::parse(&catalog, scenario_guard_1, &["ore"]).expect("valid scenario 1");
    run_one_scenario(&catalog, item, &aic1);

    make_guard!(scenario_guard_2);
    let aic2 = AicInputs::parse(&catalog, scenario_guard_2, &["ore"]).expect("valid scenario 2");
    run_one_scenario(&catalog, item, &aic2);
}
