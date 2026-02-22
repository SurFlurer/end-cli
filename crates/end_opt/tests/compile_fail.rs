#[test]
fn cross_result_node_ids_do_not_typecheck() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/cross_result_node_id.rs");
}
