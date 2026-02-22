use aic_two_brand_lab::{AicInputs, Catalog, OptimizationResult, build_report};

fn cross_mix_report<'cid, 'sid1, 'sid2>(
    catalog: &Catalog<'cid>,
    aic_1: &AicInputs<'cid, 'sid1>,
    result_2: &OptimizationResult<'cid, 'sid2, '_>,
) {
    // Intentional misuse:
    // result_2 is produced from scenario 'sid2, but rendered against aic_1 ('sid1).
    let _ = build_report(catalog, aic_1, result_2);
}

fn main() {
    let _ = cross_mix_report;
}
