# 内存布局优化计划（#plan）

## 1. 目标与约束（严格定义）

### 1.1 目标
- 在**不改变求解数学语义与 CLI 结果**的前提下，降低内存占用（优先关注求解阶段峰值内存与短期分配压力）。
- 优先处理“单位改动收益高”的部分：小集合大量 `HashMap/BTree*`、可避免的 padding、重复中间结构。

### 1.2 约束

- 允许**中度模型结构调整**（`AicInputs`/`OutpostInput` 的 `HashMap` 改为 `ItemU32Map` / `VecMap<ItemId, u32>`）。
- 新增依赖 `smallvec`
- 不使用 `unsafe`、`repr(packed)`、手工未对齐读写。
- `ItemId`/`FacilityId` 继续保持“仅在同一 `Catalog` 内有意义”的边界，不引入跨 catalog 可比性假设。

## 2. 调研结果

### 2.1 运行基线
- 数据规模（内置 catalog）：
  - `items=98`
  - `machines=13 (+ thermal_bank=1)`
  - `recipes=121`
  - `power_recipes=5`
- 当前 `aic.toml`：
  - `outposts=2`
  - `supply_items=3`
  - `sum_prices=13`
- 本地峰值 RSS（`target/debug/end-cli solve`）：
  - `Maximum resident set size = 19708 KB`

### 2.2 类型尺寸（`RUSTC_BOOTSTRAP=1 cargo rustc -p end-opt --lib -- -Zprint-type-sizes`）
- `types::StageSolution`: `192 B`
- `types::OptimizationResult`: `384 B`
- `end_model::Catalog`: `200 B`
- `end_model::OutpostInput`: `128 B`
- `end_model::FacilityDef`: `88 B`（含 `end padding: 7 bytes`）
- `solver::RecipeVars`: `88 B`
- `solver::OutpostVars`: `40 B`（含 `end padding: 4 bytes`）
- `solver::PowerVars`: `32 B`（含 `end padding: 4 bytes`）
- `types::SaleValue`: `24 B`（含 `end padding: 4 bytes`）
- `types::RecipeUsage`: `24 B`（含 `end padding: 4 bytes`）
- `types::ThermalBankUsage`: `24 B`

### 2.3 关键瓶颈判断
1. `solver::RecipeVars.net` 当前是 `HashMap<ItemId, f64>`，但每个配方平均仅约 `2.52` 个条目（最大 `3`）。
   - 改为 `SmallVec<[(ItemId, f64); 4]>`
2. `solve_stage` 使用 `BTreeSet<ItemId>` + `BTreeMap<ItemId, Expression>` 组织平衡约束。
   - 在同一 `Catalog` 内，`ItemId` 可用 `item.as_u32() as usize` 作为稠密索引，能改为按索引 `Vec` 以降低分配与节点开销。
3. `usize` 索引导致部分输出结构有 padding（`SaleValue`/`RecipeUsage` 24B）。
   - 虽然可能提升有限，但是顺手优化一下
4. `FacilityDef.power_w: Option<u32>` 占 `8B`（无 niche 压缩）。
   - `Option<NonZeroU32>`
5. `ItemId::as_u32` / `FacilityId::as_u32` 文档注释目前偏“诊断导向”，未明确索引场景。
   - 补充注释：可在同一 `Catalog` 内作为索引使用，并明确连续编号不变式与跨 catalog 不可混用。

## 3. 执行 TODO

### Phase 0: 基线与护栏
- [ ] 固化一条内存基线命令（debug/release 各一条），记录到 `docs/`。
- [ ] 增加最小化回归护栏：结果一致性（收益、机器数、电力约束）快照测试不变。

### Phase 1: 
- [x] 将 `solver::RecipeVars.net: HashMap<ItemId, f64>` 改为 `SmallVec`。
- [x] 将 item 平衡构建从 `BTreeSet/BTreeMap` 改为按 `ItemId` 索引的稠密容器。
- [x] 将报告层索引字段（`outpost_index` / `recipe_index` / `power_recipe_index`）从 `usize` 改为 `u32` 并newtype。
- [x] 调整 `FacilityDef.power_w` 表达方式以消除可避免 padding。
- [x] `AicInputs`/`OutpostInput` 的 `HashMap` 改为 `ItemU32Map`（底层 `VecMap<ItemId, u32>`）
- [x] 更新 `ItemId::as_u32` / `FacilityId::as_u32` 注释：增加“同一 `Catalog` 内可作为索引”用法，明确 `0..len` 连续编号不变式与跨 catalog 不可混用。

### Phase 2: 验收
- [x] `cargo nextest run --workspace`
- [x] `cargo clippy --workspace --all-targets`
- [ ] 对比 Phase 0 基线，确认峰值内存或分配次数有可观下降。
- [ ] 给出“改动前后”对比：
  - 峰值 RSS
  - 关键结构体尺寸变化
  - 结果一致性说明

结论：优化了个寂寞

- release RSS 对比（同命令、同 aic.toml，各 3 次）
   - 改前：17384 / 17612 / 17484 kB
   - 改后：17484 / 17480 / 17388 kB
   - 均值差：-42.7 kB（非常小，接近噪声）
- init（不跑求解）release RSS 约 9600 kB，而 solve 约 17450 kB。
   说明求解阶段额外吃掉大约 7~8 MB，这部分主要在 good_lp + HiGHS 及模型内部结构，不在你这次优化的对象上。
- 你优化的结构体确实变小了（如 OutpostInput 128->104、AicInputs 80->56、SaleValue/RecipeUsage 24->16），但这些对象数量本来就不大。
- 同时 RecipeVars 从 88->112（因为 SmallVec 内联容量）会抵消一部分收益。
