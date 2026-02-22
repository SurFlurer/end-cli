# aic-two-brand-lab

最小实验：验证双品牌生命周期在主链路上的可行性。

- `Catalog<'cid>` / `ItemId<'cid>`：catalog 维度
- `AicInputs<'cid, 'sid>` / `OutpostId<'sid>`：scenario 维度
- `OptimizationResult<'cid, 'sid, 'rid>` / `LogisticsNodeId<'rid>`：result 维度

示例代码使用普通函数 + `Guard<'id>` 参数，不用 `impl for` / HRTB。

## 覆盖映射（对照现有工程用例）

1. `load_aic` 风格 parse（key -> item id，绑定 catalog）
   - 实验入口：`AicInputs::parse`（`src/lib.rs`）
2. `run_two_stage` 风格求解（`Catalog + AIC` 同时参与，输出带 outpost id）
   - 实验入口：`run_two_stage`（`src/lib.rs`）
3. `report/web` 风格消费（`inputs.outpost(result.outpost_id)`）
   - 实验入口：`build_report`（`src/lib.rs`）
4. 同一 catalog 的 item id 可跨多个 scenario 复用
   - 演示：`src/main.rs`
5. 跨 scenario 混用 outpost id 应编译失败
   - 例子：`examples/cross_aic_outpost_misuse.rs`
6. 跨 scenario 混用 result 与 inputs 应编译失败
   - 例子：`examples/cross_aic_report_misuse.rs`
7. 跨 catalog 混用 item id 应编译失败
   - 例子：`examples/cross_catalog_item_misuse.rs`
8. 跨 result 混用 logistics node id 应编译失败
   - 例子：`examples/cross_result_node_misuse.rs`

## 运行

1. 正常路径（应通过）：

```bash
cargo check
```

2. 故意误用 1（应编译失败）：

```bash
cargo check --example cross_aic_outpost_misuse
```

3. 故意误用 2（应编译失败）：

```bash
cargo check --example cross_aic_report_misuse
```

4. 故意误用 3（应编译失败）：

```bash
cargo check --example cross_catalog_item_misuse
```

5. 故意误用 4（应编译失败）：

```bash
cargo check --example cross_result_node_misuse
```
