use end_opt::{LogisticsNodeId, OptimizationResult};

fn use_result_node<'cid, 'sid, 'rid>(
    _result: &OptimizationResult<'cid, 'sid, 'rid>,
    _node: LogisticsNodeId<'rid>,
) {
}

fn cross_mix_result_node<'cid, 'sid, 'rid1, 'rid2>(
    result_2: &OptimizationResult<'cid, 'sid, 'rid2>,
    node_1: LogisticsNodeId<'rid1>,
) {
    use_result_node(result_2, node_1);
}

fn main() {
    let _ = cross_mix_result_node;
}
