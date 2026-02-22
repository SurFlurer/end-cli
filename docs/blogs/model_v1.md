## 1. 集合（Sets）

- 物品：$i \in I$，$|I|=m$
- 配方：$r \in R$
- outpost：$o \in O$，$|O|=k$
- 机器类型：$t \in T$
- 电池类型（可被热容池消耗以发电）：$b \in B \subseteq I$

## 2. 参数（Parameters）

### 2.1 物料与配方
- $s_i \ge 0$：每分钟外部供给
- $c_i \ge 0$：每分钟外部消耗（如送货委托从仓库提取）
- $a_{r,i}$：配方净变化
  - $a_{r,i} > 0$ 产出，$a_{r,i} < 0$ 消耗

### 2.2 贸易
- $p_{o,i} \ge 0$：outpost $o$ 对物品 $i$ 的收购单价
- $C_o \ge 0$：outpost $o$ **每小时**最大交易额

### 2.3 机器产能与耗电
- 机器类型 $t$ 的机器可以生产配方集合 $R_t$，每台机器只能专门生产一个配方。
- $R_t \subseteq R$：类型 $t$ 机器可生产的配方集合
- $u_{t,r} \ge 0$：若一台类型 $t$ 机器专门跑配方 $r$，其每分钟最大吞吐
  - 约定：若 $r \notin R_t$，则 $u_{t,r}=0$
- $w_t \ge 0$：类型 $t$ 机器的功率消耗（瓦）
- $P^\text{core} = 200$: 核心免费送 200 瓦电力
- $P^\text{ext} \ge 0$: 生产机器外的其他电力消耗

### 2.4 热容池（Thermal Bank）
- 热容池自身不耗电，只发电；电池消耗计入物料守恒。
- 在该模型中热容池必须持续喂入同一类型电池以稳定输出功率。事实上，游戏电网存在容量，允许启停热容池。
- 对每个 $b \in B$：
  - $P_b \ge 0$：每台热容池持续喂入电池 $b$ 时输出功率（瓦）
  - $D_b > 0$：电池 $b$ 在热容池中持续发电秒数

## 3. 决策变量（Decision Variables）

### 3.1 流量变量（连续）
- $x_r \ge 0$：每分钟执行配方 $r$ 次数
- $q_{o,i} \ge 0$：每分钟卖给 outpost $o$ 的物品 $i$ 数量

### 3.2 离散产线设计变量（整数）
- $M_t \in \mathbb{Z}_{\ge 0}$：类型 $t$ 机器总台数
- $Y_{t,r} \in \mathbb{Z}_{\ge 0}\quad \forall t\in T,\ \forall r\in R_t$：类型 $t$ 中专门分配给配方 $r$ 的机器台数
- $Z_b \in \mathbb{Z}_{\ge 0}$：**持续**喂入电池 $b$ 的热容池台数

### 3.3 Stage 2 slack 变量（连续）
- $s^{P}\ge 0$：电力富余（瓦）
- $\tilde q_{o,i}\ge 0$：超出 outpost 预算上限的每分钟虚拟销量
- $s^{\$}\ge 0$：每分钟虚拟成交额

## 4. 目标（Two-stage Objective）

### Stage 1：最大化每分钟收入
$$
R^* = \max \ \text{Rev} \;=\; \max \ \sum_{o \in O}\sum_{i \in I} p_{o,i}\, q_{o,i}
$$

### Stage 2：在 Stage 1 最优解集合中，按用户选择优化目标

真实收入不退化约束（只统计真实销量 $q$）：
$$
\sum_{o\in O}\sum_{i\in I} p_{o,i}\, q_{o,i} \;\ge\; R^*
$$

可选目标（由前端/CLI 选择其一）：

- `min_machines`：最小化机器数量
$$
\min\ N^{mach} \;=\; \min\ \left(\sum_{t \in T} M_t \;+\; \sum_{b \in B} Z_b\right)
$$

- `max_power_slack`：最大化电力富余
$$
\max\ s^{P}
$$

- `max_money_slack`：去除预算上限，最大化每分钟虚拟成交额，仍保持真实收入不退化
$$
\max\ s^{\$} \;=\; \max\ \sum_{o\in O}\sum_{i\in I} p_{o,i}\, \tilde q_{o,i}
$$

- `weighted`：加权目标
$$
\min\ \alpha\,N^{mach} \;-\; \beta\,\hat s^{P} \;-\; \gamma\,\hat s^{\$}
$$
其中
$$
\hat s^{P}=\frac{s^{P}}{S^{P}_{\max}+\epsilon},\qquad
\hat s^{\$}=\frac{s^{\$}}{S^{\$}_{\max}+\epsilon}
$$
$S^{P}_{\max}$ 可由 `max_power_slack` 的 Stage 2 最优值给出，$S^{\$}_{\max}$ 可由 `max_money_slack` 的 Stage 2 最优值给出，$\epsilon>0$ 为数值稳定用的小常数。

## 5. 约束（Constraints）

### 5.1 物品守恒（每分钟）

- 对所有 $i \in I \setminus B$：
$$
s_i \;+\; \sum_{r \in R} a_{r,i}\, x_r \;-\; c_i \;-\; \sum_{o \in O} \left(q_{o,i}+\tilde q_{o,i}\right) \;\ge\; 0
$$

- 对所有 $b \in B$：
$$
s_b \;+\; \sum_{r \in R} a_{r,b}\, x_r \;-\; c_b \;-\; \sum_{o \in O} \left(q_{o,b}+\tilde q_{o,b}\right) \;-\; \frac{60}{D_b}\, Z_b \;\ge\; 0
$$

### 5.2 outpost 每小时交易额上限（只约束真实销量 $q$）
对所有 $o \in O$：
$$
\sum_{i \in I} p_{o,i}\, q_{o,i} \;\le\; \frac{C_o}{60}
$$

### 5.3 一台机器只能做一个配方（不允许闲置）
对所有 $t \in T$：
$$
\sum_{r \in R_t} Y_{t,r} \;=\; M_t
$$

### 5.4 配方执行速率受分配机器吞吐限制
对所有 $r \in R$：
$$
x_r \;\le\; \sum_{t \in T:\ r \in R_t} u_{t,r}\, Y_{t,r}
$$

### 5.5 电力平衡

总耗电功率：
$$
P^{use} \;=\; \sum_{t \in T} w_t\, M_t \;+\; P^\text{ext}
$$

每台持续喂入电池 $b$ 的热容池稳定输出 $P_b$，总发电功率：
$$
P^{gen} \;=\; \sum_{b \in B} P_b\, Z_b \;+\; P^\text{core}
$$

电力不短缺：
$$
P^{gen} \;\ge\; P^{use}
$$

电力 slack 定义（两种写法选一种）：

- 等式定义：
$$
s^{P} \;=\; P^{gen} \;-\; P^{use}
$$

- 或者不等式定义（避免等式；在最大化 $s^{P}$ 时会自动取等）：
$$
s^{P} \;\le\; P^{gen} \;-\; P^{use},\qquad s^{P}\ge 0
$$

### 5.6 每分钟虚拟成交额定义
$$
s^{\$} \;=\; \sum_{o\in O}\sum_{i\in I} p_{o,i}\, \tilde q_{o,i}
$$