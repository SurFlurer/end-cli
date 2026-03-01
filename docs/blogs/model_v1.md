## 1. 集合

- 物品：$i \in I$，$|I|=m$
- 配方：$r \in R$
- 哨站：$o \in O$，$|O|=k$
- 机器类型：$t \in T$
- 热能池燃料（电池）：$b \in B \subseteq I$

## 2. 参数

### 2.1 物料与配方
- $s_i \ge 0$：每分钟外部供给
- $c_i \ge 0$：每分钟外部消耗
- $a_{r,i}$：配方净变化（$a_{r,i}>0$ 产出，$a_{r,i}<0$ 消耗）

### 2.2 贸易
- $p_{o,i} \ge 0$：哨站 $o$ 对物品 $i$ 的收购单价
- $C_o \ge 0$：哨站 $o$ 每小时最大交易额

### 2.3 机器产能与耗电
- $R_t \subseteq R$：类型 $t$ 机器可生产的配方集合
- $u_{t,r} \ge 0$：一台运行配方 $r$ 的类型 $t$ 机器的每分钟最大吞吐（若 $r \notin R_t$，则 $u_{t,r}=0$）
- $w_t \ge 0$：类型 $t$ 机器的功率消耗（瓦）

### 2.4 热能池
- 对每个 $b \in B$：
  - $P_b \ge 0$：每台持续喂入电池 $b$ 的热能池输出功率（瓦）
  - $D_b > 0$：电池 $b$ 在热能池中持续发电秒数

### 2.5 电力系统开关与外部电力
- $\kappa^{P} \in \{0,1\}$：电力建模启用标志（$\kappa^{P}=1$ 启用，$\kappa^{P}=0$ 禁用）
- $P^{\text{ext-gen}} \ge 0$：外部稳定发电功率（当 $\kappa^{P}=1$ 时有效）
- $P^{\text{ext-use}} \ge 0$：系统外稳定耗电功率（当 $\kappa^{P}=1$ 时有效）

### 2.6 目标权重
- $\alpha, \beta, \gamma \ge 0$：用户指定的加权目标系数（用于 `weighted` 模式）

## 3. 决策变量

### 3.1 连续变量
- $x_r \ge 0$：每分钟执行配方 $r$ 的次数
- $q_{o,i} \ge 0$：每分钟卖给哨站 $o$ 的物品 $i$ 数量

### 3.2 整数变量
- $M_t \in \mathbb{Z}_{\ge 0}$：类型 $t$ 机器总台数
- $Y_{t,r} \in \mathbb{Z}_{\ge 0} \quad \forall t\in T,\ \forall r\in R_t$：类型 $t$ 中专门分配给配方 $r$ 的机器台数
- $Z_b \in \mathbb{Z}_{\ge 0}$：持续喂入电池 $b$ 的热能池台数（仅当 $\kappa^{P}=1$ 时存在）

### 3.3 第二阶段富余变量（用于特定目标）
- $s^{P} \ge 0$：电力富余（瓦）（仅当 $\kappa^{P}=1$ 时存在）
- $\tilde q_{o,i} \ge 0$：超出哨站预算上限的每分钟虚拟销量
- $s^{\$} \ge 0$：每分钟虚拟成交额

## 4. 目标函数（两阶段优化）

### 第一阶段：最大化每分钟收入
$$
R^* = \max \; \sum_{o \in O}\sum_{i \in I} p_{o,i}\, q_{o,i}
$$

### 第二阶段：在第一阶段最优解集合中，按以下之一优化

保持第一阶段最优收入：
$$
\sum_{o\in O}\sum_{i\in I} p_{o,i}\, q_{o,i} \;\ge\; R^*
$$

可选目标：

- **最小化机器总数**（`min_machines`）
  $$
  \min \; N^{\text{mach}} = \min \; \left( \sum_{t \in T} M_t \;+\; \kappa^{P}\sum_{b \in B} Z_b \right)
  $$

- **最大化电力富余**（`max_power_slack`，仅当 $\kappa^{P}=1$）
  $$
  \max \; s^{P}
  $$

- **最大化虚拟成交额**（`max_money_slack`）
  $$
  \max \; s^{\$} = \max \; \sum_{o\in O}\sum_{i\in I} p_{o,i}\, \tilde q_{o,i}
  $$

- **加权组合**（`weighted`）
  $$
  \min \; \alpha\,N^{\text{mach}} \;-\; \beta\,\hat s^{P} \;-\; \gamma\,\hat s^{\$}
  $$
  其中
  $$
  \hat s^{P} = \frac{s^{P}}{S^{P}_{\max}+\epsilon},\qquad
  \hat s^{\$} = \frac{s^{\$}}{S^{\$}_{\max}+\epsilon}
  $$
  $S^{P}_{\max}$、$S^{\$}_{\max}$ 分别为 `max_power_slack` 和 `max_money_slack` 单独优化得到的最优值，$\epsilon>0$ 为小常数。

  当 $\kappa^{P}=0$ 时，$\beta=0$。

## 5. 约束条件

### 5.1 物品守恒

- 对非电池物品 $i \in I \setminus B$：
  $$
  s_i \;+\; \sum_{r \in R} a_{r,i}\, x_r \;-\; c_i \;-\; \sum_{o \in O} \left(q_{o,i}+\tilde q_{o,i}\right) \;\ge\; 0
  $$

- 对电池物品 $b \in B$：
  $$
  s_b \;+\; \sum_{r \in R} a_{r,b}\, x_r \;-\; c_b \;-\; \sum_{o \in O} \left(q_{o,b}+\tilde q_{o,b}\right) \;-\; \frac{60}{D_b}\, Z_b \;\ge\; 0
  $$

### 5.2 哨站每小时交易额上限
对所有 $o \in O$：
$$
\sum_{i \in I} p_{o,i}\, q_{o,i} \;\le\; \frac{C_o}{60}
$$

### 5.3 一台机器只能做一个配方
对所有 $t \in T$：
$$
\sum_{r \in R_t} Y_{t,r} \;=\; M_t
$$

### 5.4 配方执行速率受分配机器吞吐限制
对所有 $r \in R$：
$$
x_r \;\le\; \sum_{t \in T:\ r \in R_t} u_{t,r}\, Y_{t,r}
$$

### 5.5 电力平衡（仅当 $\kappa^{P}=1$）

总耗电功率：
$$
P^{use} \;=\; P^{\text{ext-use}} \;+\; \sum_{t \in T} w_t\, M_t
$$

总发电功率：
$$
P^{gen} \;=\; P^{\text{ext-gen}} \;+\; \sum_{b \in B} P_b\, Z_b
$$

电力不短缺：
$$
P^{gen} \;\ge\; P^{use}
$$

电力富余定义（自动取等当最大化 $s^{P}$）：
$$
s^{P} \;\le\; P^{gen} \;-\; P^{use},\qquad s^{P}\ge 0
$$

**注**：当 $\kappa^{P}=0$ 时，变量 $Z_b$、$s^{P}$ 及电力平衡约束（5.5）被移除；加权目标中的 $\beta$ 视为 0。

### 5.6 每分钟虚拟成交额定义
$$
s^{\$} \;=\; \sum_{o\in O}\sum_{i\in I} p_{o,i}\, \tilde q_{o,i}
$$

