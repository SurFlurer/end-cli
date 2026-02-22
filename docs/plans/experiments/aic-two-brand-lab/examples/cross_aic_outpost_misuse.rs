use aic_two_brand_lab::{AicInputs, OutpostId};

fn use_outpost<'cid, 'sid>(_aic: &AicInputs<'cid, 'sid>, _outpost: OutpostId<'sid>) {}

fn cross_mix<'cid, 'sid1, 'sid2>(aic_2: &AicInputs<'cid, 'sid2>, outpost_1: OutpostId<'sid1>) {
    // Intentional misuse:
    // outpost_1 uses 'sid1, but aic_2 expects 'sid2.
    use_outpost(aic_2, outpost_1);
}

fn main() {
    let _ = cross_mix;
}
