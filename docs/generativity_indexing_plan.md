# `end_opt` Generativity 落地与 `indexing_slicing` 消警计划

## 0. 模式声明（`#plan`）

本文件仅包含调研结论与执行 TODO，不包含实质性代码改动。

## 1. 目标（严格复述）

1. 在不改变求解语义的前提下，消除 `crates/end_opt/src/solver.rs` 当前 5 处 `clippy::indexing_slicing` warning。
2. 以可维护方式落地 <https://arhan.sh/blog/the-generativity-pattern-in-rust/> 的核心思想：
   1. 使用“新鲜（fresh）且不可伪造”的生命周期 token；
   2. 用该 token 绑定“已校验索引”和“被索引容器”；
   3. 把越界风险收敛到类型内部，而非业务调用处。
3. 满足仓库约束：`parse, don't validate`；业务代码不引入 `allow`/`panic`/`expect`/`unsafe`；不以 `#[allow(clippy::indexing_slicing)]` 掩盖问题。
4. 执行后通过 `cargo make done`。

## 2. 已确认约束

1. 你当前要求是 `#plan`，所以只调研和产出计划，不做实质性改动。
2. 当前 warning 由 `cargo clippy -p end-opt -- -W clippy::indexing_slicing` 复现，位置如下：
   1. `crates/end_opt/src/solver.rs:172`
   2. `crates/end_opt/src/solver.rs:181`
   3. `crates/end_opt/src/solver.rs:190`
   4. `crates/end_opt/src/solver.rs:199`
   5. `crates/end_opt/src/solver.rs:352`
3. 现状有运行时校验 `validate_aic_item_ids`（`crates/end_opt/src/solver.rs:380`），但 `item_balance` 仍使用裸 `Vec` 索引。

## 3. 调研结论

### 3.1 根因

1. `ItemId` 是 `u32` newtype，`index()` 直接返回 `usize`（`crates/end_model/src/catalog/model/types.rs:17`）。
2. `AicInputs` 中的 `ItemId` 不与具体 `Catalog` 生命周期绑定（`crates/end_model/src/aic_input.rs:190`）。
3. 求解器中 `item_balance` 是 `Vec<Expression>`，被多个来源的 `ItemId` 直接索引。

结论：当前安全性依赖“先验约定 + 运行时检查”，编译器无法证明“索引一定合法”，所以 clippy 报警是合理的。

### 3.2 与 Generativity 模式的对应关系

根据文章与 `generativity` crate 文档，落地点可抽象为：

1. 在一次求解上下文里生成一个新鲜 token（`'id`）。
2. 只允许持有该 token 的 `BrandedItemId<'id>` 访问 `ItemBalance<'id>`。
3. 外部输入的 `ItemId` 在“解析阶段”转换为 `BrandedItemId<'id>`，失败即报错。
4. 业务路径不再处理“可能越界”分支；越界处理集中在 parse 层。

### 3.3 可行实现路径

#### 路径 A（推荐）：引入 `generativity` crate

1. 优点：
   1. 直接使用成熟 API（`make_guard!` + `Id<'id>`），降低手写不变型生命周期的出错概率；
   2. 代码意图更清晰，后续维护成本更低。
2. 代价：
   1. 增加一个小依赖；
   2. `solver.rs` 需要一次结构化重排（主要是类型参数 `'id` 传递）。

#### 路径 B：手写文章中的 invariance 模式（不加依赖）

1. 优点：
   1. 零第三方依赖；
   2. 对模式机制理解更深。
2. 风险：
   1. `PhantomData<fn(&'id ()) -> &'id ()>` 等细节易错；
   2. 代码评审成本高于路径 A。

#### 路径 C（不推荐）：改成 `get/get_mut` + 错误分支

1. 能消 clippy，但不满足“parse, don't validate”的目标；
2. 会把防御分支散落到业务热路径，违背当前项目风格。

## 4. 建议方案（供 `#go` 执行）

选择路径 A，并把改动限制在 `end_opt` 内部，不改公开 API。

### 4.1 设计草图

1. 新增内部类型模块（建议文件：`crates/end_opt/src/item_brand.rs`）：
   1. `BrandedItemId<'id>`：已与 token 绑定且保证在 `item_count` 范围内；
   2. `ItemBalance<'id>`：只接受 `BrandedItemId<'id>` 读写；
   3. `ParsedAic<'id>`：把 `AicInputs` 中会参与索引的项一次性解析为 branded 版本。
2. 在求解入口创建 token，并在同一 token 作用域内完成：
   1. 解析输入；
   2. 构建变量；
   3. 构建/读取 `item_balance`。
3. `unsafe`（如果需要）只允许存在于类型内部，并写明不变量来源；业务调用点保持纯安全 API。

### 4.2 预期改动文件

1. `Cargo.toml`（workspace 依赖，若采用路径 A）。
2. `crates/end_opt/Cargo.toml`（添加 `generativity` 依赖引用）。
3. `crates/end_opt/src/solver.rs`（使用 branded 类型替代裸索引）。
4. `crates/end_opt/src/lib.rs`（若新增模块需 `mod` 声明）。
5. 可选：`crates/end_opt/tests/two_stage_regression.rs`（补一条“解析阶段报错定位准确”的回归用例）。

## 5. 执行 TODO（`#go` 时按序）

1. [ ] 引入 `generativity` 依赖并编译通过。
2. [ ] 新增 `BrandedItemId<'id>` 与 `ItemBalance<'id>`，封装索引读写 API。
3. [ ] 实现 `ParsedAic<'id>`，把 `validate_aic_item_ids` 逻辑迁移为 parse 阶段类型化转换。
4. [ ] 改造 `solver.rs` 的 5 处索引为 branded API 调用，删除裸 `Vec[idx]` 访问。
5. [ ] 保持错误文案与当前行为兼容（尤其是 `supply_per_min`/`outposts[*].prices` 的报错来源）。
6. [ ] 运行 `cargo clippy -p end-opt -- -W clippy::indexing_slicing`，确认 0 warning。
7. [ ] 运行 `cargo make done`，确认全量通过。
8. [ ] 产出变更说明：新不变量、为何更安全、为何无行为回归。

## 6. 需要你确认的选项

1. 依赖策略：
   1. 选项 A（推荐）：使用 `generativity` crate；
   2. 选项 B：手写 invariance，不加依赖。
2. 测试策略：
   1. 选项 A（推荐）：只补与 parse-branding 直接相关的最小回归；
   2. 选项 B：仅跑现有测试，不新增用例。
3. 改动范围：
   1. 选项 A（推荐）：先只改 `end_opt`；
   2. 选项 B：同步把 `end_model` 的 `ItemId` 体系也品牌化（改动大，不建议首轮做）。

## 7. 进阶方案：`Catalog` 与 `Id` 生命周期强绑定

你提出的“更强约束”可行，但建议按两层 API 设计，避免把调用端复杂度一次拉满。

### 7.1 核心目标

1. 让 `ItemId` / `FacilityId` / `RecipeId` 不能脱离所属 `Catalog` 上下文使用。
2. 禁止跨 catalog 混用 id（编译期报错，而非运行时报错）。
3. 把“可能无效 id”限制在 parse 边界，业务层只处理已解析后的强类型。

### 7.2 推荐结构（两层）

1. 保留拥有数据的 `Catalog`（无生命周期参数）。
2. 新增作用域化视图 `CatalogCtx<'a, 'id>`：
   1. `&'a Catalog` + fresh brand token（`Id<'id>`）；
   2. 所有“会产出 id”的 API 只在 `CatalogCtx` 上提供。
3. `ItemId<'id>` 等 branded id：
   1. 内部保存已校验索引；
   2. 外部不暴露任意构造。
4. 提供闭包入口，确保 `'id` 新鲜且不可伪造：
   1. `Catalog::with_ctx(|ctx| { ... })`；
   2. `ctx` 内可解析 AIC、求解、渲染；
   3. 离开闭包后 `ItemId<'id>` 无法逃逸。

### 7.3 对现有模块的影响

1. `end_io`：
   1. 现有 `load_aic(path, &Catalog) -> AicInputs` 解析逻辑需要改为在 `CatalogCtx` 内产出 `AicInputs<'id>`；
   2. unknown key 的报错仍在 IO 层完成，符合 parse-first。
2. `end_opt`：
   1. `SolveInputs<'id>` / `StageSolution<'id>` 可能引入生命周期；
   2. 求解器内可彻底去掉 `validate_aic_item_ids` 这类跨 catalog 防御检查。
3. `end_report` 与 `end_cli`：
   1. 如果报告对象继续携带 id，则 `build_report` 也需在同一 `CatalogCtx<'id>` 作用域运行；
   2. 这是可接受的（CLI 当前就是“加载 -> 求解 -> 渲染”同一流程）。

### 7.4 一个关键取舍（必须确认）

1. 方案 A（最强约束，推荐）：
   1. 结果类型保留 branded id（`StageSolution<'id>`）；
   2. 强约束最大化，但 API 会出现 `'id`。
2. 方案 B（兼容优先）：
   1. 内部求解全程 branded；
   2. 对外结果降级成 key/string 或稳定整数；
   3. 公共 API 基本不带生命周期，但约束弱于 A（边界处有一次显式降级）。

### 7.5 分阶段落地建议

1. Phase 1：先完成第 4~6 节（`end_opt` 局部 branded，消除 5 处 warning）。
2. Phase 2：引入 `CatalogCtx<'id>` + `AicInputs<'id>`，打通 `end_io -> end_opt`。
3. Phase 3：决定输出层走 7.4-A 还是 7.4-B，并统一 `end_report/end_cli` 签名。
