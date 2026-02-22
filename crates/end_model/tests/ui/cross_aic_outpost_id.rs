use end_model::{AicInputs, OutpostId};

fn cross_mix_outpost<'cid, 'sid1, 'sid2>(aic_1: &AicInputs<'cid, 'sid1>, outpost_2: OutpostId<'sid2>) {
    let _ = aic_1.outpost(outpost_2);
}

fn main() {
    let _ = cross_mix_outpost;
}
