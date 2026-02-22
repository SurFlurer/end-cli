use aic_two_brand_lab::{AicInputs, LogisticsNodeId, OptimizationResult, use_result_node};

fn cross_mix_result_node<'cid, 'sid, 'rid1, 'rid2>(
    aic: &AicInputs<'cid, 'sid>,
    result_2: &OptimizationResult<'cid, 'sid, 'rid2>,
    node_1: LogisticsNodeId<'rid1>,
) {
    // Intentional misuse:
    // node_1 is from result 'rid1, but consumed with result_2 ('rid2).
    use_result_node(aic, result_2, node_1);
}

fn main() {
    let _ = cross_mix_result_node;
}
