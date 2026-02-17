> 总体思路：按物品分解。对每个物品 $i$ 单独运行一个确定性的 Best-Fit 启发式分配器。

### 2.1 从阶段 2 解展开机器粒度节点

记配方 $r$ 的总执行速率为 $x_r$（次/min），机器数为 $y_r$，单机上限为 $u_r = 60 / time_r$。  
将其拆到每台机器 $m = 1..y_r$：

$$
\rho_{r,m} = \min\left(u_r,\ \max\left(x_r - (m-1)u_r,\ 0\right)\right)
$$

则有：
- $0 \le \rho_{r,m} \le u_r$
- $\sum_{m=1}^{y_r} \rho_{r,m} = x_r$

记配方净变化为 $a_{r,i}$（和 `model_v1` 一致，产出为正，消耗为负）：

- 若 $a_{r,i} > 0$，机器 $(r,m)$ 对物品 $i$ 的供给量为 $a_{r,i} \cdot \rho_{r,m}$。
- 若 $a_{r,i} < 0$，机器 $(r,m)$ 对物品 $i$ 的需求量为 $(-a_{r,i}) \cdot \rho_{r,m}$。

热容池燃料需求：对 power recipe $p$（燃料物品 $ing(p)$，时长 $D_p$，台数 $z_p$）：

$$
\delta_p = \frac{60}{D_p}
$$

每台 bank 产生一个需求点，需求量 $\delta_p$。

### 2.2 集合（Sets）

对固定物品 $i$：

- 供给点集合：$s \in S_i$
  - 外部供给点（可选）；
  - 所有对 $i$ 有正净产出的机器实例点。
- 需求点集合：$d \in D_i$
  - 所有消耗 $i$ 的机器实例点；
  - 售卖 $i$ 的 outpost 点；
  - 消耗 $i$ 的热容池实例点。

### 2.3 参数（Parameters）

- $U_{i,s} > 0$：供给点 $s$ 的最大可供给流量（units/min）。
- $V_{i,d} > 0$：需求点 $d$ 的必须满足流量（units/min）。
- $\varepsilon > 0$：浮点容差（推荐 `1e-9`）。

可行性前提（来自阶段 2 物料守恒）：

$$
\sum_{s \in S_i} U_{i,s} \ge \sum_{d \in D_i} V_{i,d}
$$

### 2.4 Best-Fit 分配规则

定义剩余量：

- $R_{i,s}$：供给点 $s$ 的剩余可供给量，初始为 $U_{i,s}$。
- $N_{i,d}$：需求点 $d$ 的剩余需求量，初始为 $V_{i,d}$。

需求处理顺序（保证稳定）：

1. 按 $V_{i,d}$ 降序（先处理大需求）。
2. 同流量下按 `DemandNodeId` 升序。

对每个需求点 $d$，循环直到 $N_{i,d} \le \varepsilon$：

1. 构造可一口气满足当前需求的候选集合  
   $C_d = \{ s \in S_i \mid R_{i,s} \ge N_{i,d} \}$。
2. 若 $C_d$ 非空，选  
   $s^\* = \arg\min_{s \in C_d}(R_{i,s}, \text{SupplyNodeId}(s))$，  
   即“能装下且最紧”的供给点（Best-Fit），分配 $q = N_{i,d}$。
3. 若 $C_d$ 为空且存在 $R_{i,s} > \varepsilon$，选剩余量最大的供给点  
   $s^\* = \arg\max_{s: R_{i,s} > \varepsilon}(R_{i,s}, -\text{SupplyNodeId}(s))$，  
   分配 $q = \min(R_{i,s^\*}, N_{i,d})$。
4. 若 $C_d$ 为空且不存在 $R_{i,s} > \varepsilon$，返回不可行错误。
5. 记一条流量边（若同一边已存在则累加）：
   $f_{i,s^\*,d} \mathrel{+}= q$。
6. 更新剩余量：
   $R_{i,s^\*} \leftarrow R_{i,s^\*} - q$，
   $N_{i,d} \leftarrow N_{i,d} - q$。

### 2.5 结果性质（Properties）

- 可行性前提成立时，算法应满足每个需求点：
$$
\sum_{s \in S_i} f_{i,s,d} = V_{i,d} \quad \forall d \in D_i
$$
- 任一供给点不超上限：
$$
\sum_{d \in D_i} f_{i,s,d} \le U_{i,s} \quad \forall s \in S_i
$$
- 由于使用启发式，连接数量不是全局最优保证，只保证确定性与可复现。

### 2.6 复杂度与终止（Complexity）

- 朴素实现每次扫描供给点，单物品时间复杂度约为
  $O(|D_i| \cdot |S_i| + K_i \cdot |S_i|)$，其中 $K_i$ 是拆分产生的额外分配轮次。
- 算法单调减少总剩余需求 $\sum_d N_{i,d}$，若可行性前提成立则必然终止。
- 若运行中出现 $N_{i,d} > \varepsilon$ 且所有 $R_{i,s} \le \varepsilon$，应返回不可行错误。

### 2.7 输出表示（Graph）

虽然求解仍按物品拆分执行，但最终 `LogisticsPlan` 输出为统一图结构：

- `nodes: Vec<LogisticsNode>`
  - `ExternalSupply { item }`
  - `RecipeMachine { recipe_index, machine }`
  - `OutpostSale { outpost_index, item }`
  - `ThermalBankFuel { power_recipe_index, bank, item }`
- `edges: Vec<LogisticsEdge>`
  - 每条边携带 `item` 与 `flow_per_min`，并引用 `from`/`to` 节点 ID。

关键不变量：

- 同一台机器（`recipe_index + machine`）在图中只对应一个 `RecipeMachine` 节点；
  不再区分“输入节点”和“输出节点”两个 ID。
- 边以 `(item, from, to)` 去重聚合，流量为正且有限。
