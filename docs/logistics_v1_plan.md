# 产线规划（物流连线最少）功能方案

> 目的：把现有两阶段 MILP 求产线规模的结果，进一步生成一个整体物流图。  
> 方法：对**每个物品**单独构建运输/分配子问题（供给点、需求点按机器粒度），在满足所有需求的前提下**最小化连接数量**，再把所有物品的结果合并成一张图。

## 0. 范围与非目标

### 0.1 目标
- 输入：`Catalog` + `AicInputs` + Stage2 的求解结果（配方执行速率、机器数、售卖量、热容池配置等）。
- 输出：
  - 每个物品的一组“物流连接”（供给点 → 需求点，带流量）。
  - 合并后的整体物流图。
  - CLI 支持导出 DOT 文件。

### 0.2 非目标（v1 明确不做）
- 不优化空间距离/布局长度，只优化“边数量”。
- 不做时序、缓冲、库存动态，只做稳态流量（units/min）。
- 不引入第二种求解策略（先只实现 MILP；必要时加规模保护报错）。


## 1. 术语与关键定义（decision-complete）

### 1.1 “连接”是什么
- **连接 = 直连边**：供给点 `s` → 需求点 `d` 的一条边。
- “计数规则”：如果某条边上该物品的流量 `f_{s,d} > 0`，则认为该边被使用，计 `1` 条连接。
- 注意：连接按“物品维度”计算；可视化时允许把多物品合并显示在同一条边 label 上（不改变计数口径）。

### 1.2 节点粒度（混合输出）
- 内部求解：按**每台机器实例**做供给/需求点。
- 输出展示：按“同配方机器组 + 速度档”聚合（但必须保留未满速/空转的区分）：
  - `Full`：满速机器
  - `Partial`：未满速机器（需要单独显示）
  - `Idle`：几乎不运行（需要单独显示）

---

## 2. 总体流水线（实现顺序）

1) **优化求解**：保持现有 `end_opt` 的两阶段模型不变，但需要补齐“全量解”输出（见第 3 节）。
2) **机器实例化**：把每个配方的 `(x_r runs/min, y_r machines)` 拆成 `y_r` 台机器实例的 `exec_rate`。
3) **逐物品运输 MILP**：对每个物品 `i` 解固定费用运输问题（最小化边数）。
4) **合并输出**：把所有物品边合并为整体图，导出 DOT。

---

## 3. end_opt 输出扩展（为物流提供“全量解”）

### 3.1 问题现状
- 当前 `StageSolution` 中 `recipes_used/top_sales` 是截断后的摘要，不足以构建物流图（物流需要所有 recipe/outpost/power_recipe 的非零量）。

### 3.2 方案（最小破坏）
- 保留现有：
  - `run_two_stage(...) -> OptimizationResult`（继续给 report 用，输出摘要）
- 新增：
  - `run_two_stage_full(...) -> OptimizationResultFull`

### 3.3 新类型（建议放 `crates/end_opt/src/types.rs`）
- `StageSolutionFull`（stage1/stage2 都要）：
  - `recipes: Vec<RecipeSolvedLine>`：每个 recipe 的 `machines`、`executions_per_min`（不截断）
  - `outpost_sales: Vec<OutpostSaleLine>`：每个 outpost、每个 item 的 `qty_per_min`（只保留 > eps 的）
  - `power_banks: Vec<PowerSolvedLine>`：每个 power_recipe 的 `banks`（只保留 > 0 的）
  - 以及现有 `StageSolution` 的核心汇总字段（revenue、power、总机器数等），保持一致。
- `OptimizationResultFull { stage1: StageSolutionFull, stage2: StageSolutionFull }`

### 3.4 兼容性
- `run_two_stage` 内部可调用 `run_two_stage_full`，然后把 full 投影/截断为旧的 `OptimizationResult`（以保证现有 CLI/report 无需大改）。

---

## 4. 机器实例化（从 recipe 总量到 machine instance）

对每个配方 `r`：
- 单机最大速率：`cap = 60 / time_s`（runs/min）
- stage2 解：`x = executions_per_min`、`y = machines`（整数）

实例化规则（装箱式，保证可执行语义）：
- `remaining = x`
- 对 `k = 1..y`：
  - `exec_k = min(cap, remaining)`
  - `remaining -= exec_k`
- 速度分类：
  - `Full`：`exec_k ≈ cap`
  - `Partial`：`0 < exec_k < cap`
  - `Idle`：`exec_k ≈ 0`

数值容差（v1 固定）：
- `exec_k <= 1e-9` 视为 0
- `|exec_k - cap| <= 1e-9` 视为满速

输出聚合要求：
- `Partial/Idle` 必须单独节点（不可与 Full 合并）。

---

## 5. 逐物品运输子问题（Fixed-charge transportation MILP）

### 5.1 对某个物品 i 的供给点/需求点构造

供给点（sources）：
- 外部供给：`ExternalSupply(i)`，供给量 `s_i`
- 机器实例：若配方对 `i` 的净产出 `net_i_per_run > 0`，则该机器实例供给量为：
  - `s = net_i_per_run * exec_rate(machine)`

需求点（demands）：
- 机器实例：若 `net_i_per_run < 0`，则需求量为：
  - `d = (-net_i_per_run) * exec_rate(machine)`
- Outpost：对每个 outpost `o`，节点 `Outpost(o)`，需求量为：
  - `d = q_{o,i}`（来自 stage2 的售卖量）
- Thermal bank 燃料：
  - 节点 `ThermalBank(p,k)`（每台热容池实例）
  - 对燃料物品 `b`，每台需求量：`60 / duration_s`

守恒与 slack：
- 需求点必须完全满足（等式）
- 供给点允许富余（≤），剩余视为“自然富余/入库/丢弃”，不额外引入 dummy sink

### 5.2 MILP 形式

集合：
- sources `S`
- demands `D`

变量：
- 流量 `f_{s,d} >= 0`
- 边启用 `y_{s,d} ∈ {0,1}`

约束：
- 需求满足：对所有 `d ∈ D`，`Σ_s f_{s,d} = demand_d`
- 供给上限：对所有 `s ∈ S`，`Σ_d f_{s,d} <= supply_s`
- Big-M：`f_{s,d} <= M_{s,d} * y_{s,d}`
  - `M_{s,d} = min(supply_s, demand_d)`（更紧）

目标：
- `min Σ_{s,d} y_{s,d}`（最小连接数量）

### 5.3 求解策略（v1）
- 只实现 MILP（你确认：先简单、后续再抽象扩展）
- 加规模保护（v1 固定阈值，超过则报错，不做降级算法）：
  - 若 `|S| * |D| > THRESHOLD`（例如 20000），报错提示后续可加筛选/近似。

---

## 6. 物流图合并与 DOT 导出

### 6.1 内部边合并（machine instance -> group）
- 内部求解边是“机器实例”级别，数量可能大。
- 输出聚合：
  - 节点 key：`(recipe_index, speed_class)`；外部供给、outpost、热容池各自独立节点。
  - 边 key：`(from_group, to_group, item)`，把相同 key 的流量求和为一条边。

### 6.2 整体图进一步合并（可读性）
- 可选：按 `(from_group, to_group)` 合并边，把多物品 label 合并显示：
  - 只显示 top N（按流量或按贡献）+ “+k more”

### 6.3 DOT 约定（v1）
- 节点：
  - recipe group 节点 label：设施名 + 配方简写 + speed_class + 台数
  - outpost 节点 label：outpost 名
  - external 节点 label：`External: <item>`
- 边：
  - label：`<item>: <flow>/min`（多物品时按约定合并）

---

## 7. 代码结构建议（v1）

新增 crate：`crates/end_logistics`
- 输入：`Catalog`、`AicInputs`、`StageSolutionFull`（用 stage2）
- 输出：
  - `LogisticsPlan`（逐物品边集合、聚合后边集合、节点集合、连接数统计）
  - `to_dot(lang: Lang, ...) -> String`

CLI 改动：`crates/end_cli`
- `solve` 子命令新增参数：
  - `--logistics-dot <FILE>`：如果提供，输出 DOT 文件

---

## 8. 测试与验收

### 8.1 单元测试（建议放 `crates/end_logistics/tests/`）
1) 最少边数基本例：每个需求点都能被一个供给点满足，期望每个 demand 恰好 1 条入边。
2) 必须拆分例：单个 demand 超过任意单一 supply，期望入边数 ≥ 2 且满足等式。
3) 富余供给例：总供给 > 总需求，期望不出现 dummy sink，边数不被富余污染。
4) 满速/非满速拆分例：`cap=10, x=15, y=2` -> 实例化 `[10,5]`，且输出聚合能区分 Full/Partial。

### 8.2 手工验收（基于当前默认 `cargo run`）
- `cargo run -q -- solve --logistics-dot logistics.dot` 能生成 DOT 文件且可渲染：
  - `dot -Tsvg logistics.dot > logistics.svg`
- 图中能看到 external/outpost/配方组/热容池节点与边。

---

## 9. v1 默认假设（写进文档/错误信息）
- 连接数按“物品维度”计数；可视化允许合并显示但不改变计数口径。
- 不做距离与布局优化。
- 稳态流量模型，不做库存与时序。
- 浮点噪声阈值：`1e-9`。

