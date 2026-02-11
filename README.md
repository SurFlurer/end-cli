# end-cli

`end-cli` 是一个用于《明日方舟：终末地》自动化生产规划的命令行求解器。  
给定外部供给（矿点）、据点收购价和预算池、基地内外的额外耗电，它会自动算出:

- 每分钟该跑哪些配方（以及跑多少）
- 各类生产机器需要多少台
- 热容池该喂哪种电池、要开几台
- 卖给哪个据点哪些货，收益最高

## 它到底在优化什么

程序使用两阶段 MILP（混合整数线性规划）模型，详细公式见 [model_v1.md](docs/model_v1.md):

1. Stage 1: 最大化每分钟总收入
2. Stage 2: 在 Stage 1 的最优收入附近，最小化机器总数（生产机器 + 热容池）

核心约束包括:

- 物料守恒（含热容池燃料消耗）
- 据点每小时交易额上限
- 配方吞吐受机器数量约束
- 总发电功率 >= 总用电功率

## 安装

### 方式一：下载 GitHub Releases 二进制文件

在 [GitHub Releases](https://github.com/sssxks/end-cli/releases) 页面下载对应平台的压缩包。解压后将 `end-cli` 可执行文件所在目录添加到系统 PATH 中即可。

### 方式二：cargo install

需要 Rust 环境，执行:

```bash
# 编译 HiGHS 需要 clang 和 cmake，Debian/Ubuntu 可用以下命令安装
sudo apt-get update && sudo apt-get install -y libclang-dev clang cmake
cargo install --git https://github.com/sssxks/end-cli end-cli
```

## 快速开始

1. 生成配置模板:

```bash
end-cli init
```

2. 编辑当前目录下的 `aic.toml`（外部供给、据点价格、据点上限、外部耗电）。

3. 运行求解:

```bash
end-cli solve
```

默认输出中文报告。英文报告可用:

```bash
end-cli solve --lang en
```

如果你看到下面这条报错:

```text
Error: aic.toml not found; run `end-cli init --aic aic.toml` to create it
```

它表示当前目录没有对应配置文件，`solve` 会直接拒绝执行。先运行 `end-cli init` 生成模板并按需修改后再求解。

## 一次真实运行示例

以下是 `end-cli solve --lang en` 的实际输出片段:

```text
Conclusion
Conclusion: with the current external supply and P^ext=300W, optimal revenue is about 739.80/min (44388/h). Line size: 35 production machines + 2 thermal banks; power margin 30W.

Trading
- Refugee Camp: 288.60/min (cap 288.60/min, 100%) Capped
- Infra Station: 451.20/min (cap 451.20/min, 100%) Capped

Power
- Generation 670W = P^core 200W + thermal banks; usage 640W = P^ext 300W + production machines; margin 30W OK

Production
- Total production machines: 35 (by facility)

Bottlenecks & Tips
- Refugee Camp is capped: producing more won't sell; prioritize higher-price products or switch/add outposts.
```

这份报告一般可按以下顺序阅读:

1. `Conclusion`: 看收益规模、机器总量、电力余量
2. `Trading`: 看哪个据点触顶、主要卖什么
3. `Power`: 看是否接近电力上限
4. `Production`: 看主要机器和配方分布
5. `Bottlenecks & Tips`: 看下一步该扩哪里

## `aic.toml` 关键字段

`end-cli init` 生成的模板大致如下:

```toml
external_power_consumption_w = 300

[supply_per_min]
"Originium Ore" = 520
"Ferrium Ore" = 160
"Amethyst Ore" = 160

[[outposts]]
key = "Refugee Camp"
money_cap_per_hour = 17316
[outposts.prices]
"SC Valley Battery" = 30
Origocrust = 1
```

可重点调整:

- `external_power_consumption_w`: 非生产机器占用的额外电力
- `supply_per_min`: 每种原料的每分钟外部供给
- `outposts[].money_cap_per_hour`: 据点每小时交易额上限
- `outposts[].prices`: 各商品在该据点的收购价

## 常用命令

```bash
# 初始化配置（覆盖已存在文件）
end-cli init --force

# 用指定配置文件求解
end-cli solve --aic aic.toml

# 指定语言
end-cli solve --lang en

# 指定数据目录（items/facilities/recipes）
end-cli solve --data-dir ./data

# 查看帮助
end-cli --help
end-cli init --help
end-cli solve --help
```
