#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
枚举「扩容反应池 (mix_pool_2) 合并配方」候选, 并计算哪些比纯普通反应池 (mix_pool_1) 更省电.

游戏机制(由用户提供):
  - 普通反应池 mix_pool_1: 50W, 一次只能跑一个反应.
  - 扩容反应池 mix_pool_2: 100W, 可同时跑多个不同反应; 中间产物在池内就地消化,
    不占用外部物料搬运. 一台扩容池有 8 个物品槽位(= 涉及的不同物品种类上限).
  - 所有 mix_pool 反应 time_s = 2s => 单台 throughput = 30/min.

模型:
  一个「合并配方」= 一组不同的单步 mix_pool_1 反应 {R_i}, 每个贡献一次.
  其净效果 net = Σ_i (products_i - ingredients_i).
  - 净流量恰为 0 的物品 = 被池内消化的中间产物(不出现在合并配方的输入/输出里).
  - 净流量 ≠ 0 的物品 = 合并配方的外部输入(负) / 外部输出(正).

耗电对比 (产出与合并配方相同净效果):
  - 用扩容池: 1 台 × 100W.
  - 用普通池: 每个反应一台 => |S| 台 × 50W  (同一组反应, 产能一致).
  对同一净效果, 还会在所有可能反应子集中找「最小普通池台数」, 给出诚实下界.
  省电 = 普通池耗电 - 扩容池耗电; 当需要 ≥3 台普通池时, 扩容池才真正省电.

用法:
  python scripts/find_expanded_crucible_merges.py [--slots N] [--max-reactions N] [--emit-toml PATH]

默认读取 crates/end_io/src/new-data/factory_recipes.toml (相对仓库根).
"""

from __future__ import annotations

import argparse
import io
import sys
import tomllib
from collections import Counter
from itertools import combinations
from pathlib import Path

# ---------------------------------------------------------------------------
# 配置
# ---------------------------------------------------------------------------

DEFAULT_SLOTS = 8            # 扩容池物品槽位数(游戏真实值)
DEFAULT_MAX_REACTIONS = 4    # 单台扩容池同时跑的最多不同反应数上限(可调)
MIX_POOL_1_POWER = 50        # 普通反应池功率
MIX_POOL_2_POWER = 100       # 扩容反应池功率
THROUGHPUT_PER_MIN = 30      # time_s=2 => 60/2

REPO_ROOT = Path(__file__).resolve().parents[1]
RECIPES_TOML = REPO_ROOT / "crates" / "end_io" / "src" / "new-data" / "factory_recipes.toml"
ITEMS_TOML = REPO_ROOT / "crates" / "end_io" / "src" / "new-data" / "factory_items.toml"


# ---------------------------------------------------------------------------
# 数据加载
# ---------------------------------------------------------------------------

def load_recipes(path: Path) -> list[dict]:
    """读取 toml, 返回所有 facility == 'mix_pool_1' 的单步反应."""
    with path.open("rb") as f:
        doc = tomllib.load(f)
    out = []
    for r in doc.get("recipes", []):
        if r.get("facility") == "mix_pool_1":
            out.append(r)
    return out


def load_item_meta(path: Path) -> dict[str, dict]:
    """item key -> {zh, fluid}."""
    with path.open("rb") as f:
        doc = tomllib.load(f)
    meta = {}
    for it in doc.get("items", []):
        meta[it["key"]] = {
            "zh": it.get("zh", it["key"]),
            "fluid": it.get("fluid", False) or it.get("gas", False),
        }
    return meta


def stack_to_counter(stack_list: list[dict]) -> Counter:
    c = Counter()
    for s in stack_list:
        c[s["item"]] += s["count"]
    return c


# ---------------------------------------------------------------------------
# 合并配方计算
# ---------------------------------------------------------------------------

class Reaction:
    """一条单步反应."""

    def __init__(self, idx: int, raw: dict):
        self.idx = idx
        self.facility = raw["facility"]
        self.time_s = raw["time_s"]
        self.ingredients = stack_to_counter(raw.get("ingredients", []))
        self.products = stack_to_counter(raw.get("products", []))
        # 净: 产品 - 原料
        self.net = Counter()
        for k, v in self.products.items():
            self.net[k] += v
        for k, v in self.ingredients.items():
            self.net[k] -= v

    def items_touched(self) -> set[str]:
        return set(self.ingredients) | set(self.products)

    def label(self) -> str:
        def fmt(c: Counter) -> str:
            return " + ".join(f"{k}×{v}" for k, v in sorted(c.items()))
        return f"{fmt(self.ingredients)} -> {fmt(self.products)}"


def merged_net(reactions: list[Reaction]) -> Counter:
    net = Counter()
    for r in reactions:
        net.update(r.net)
    # 去掉恰好为 0 的(浮点安全: 这里全是整数)
    return Counter({k: v for k, v in net.items() if v != 0})


def external_items(net: Counter) -> tuple[list[str], list[str]]:
    """返回 (外部输入, 外部输出)."""
    inputs = sorted([k for k, v in net.items() if v < 0])
    outputs = sorted([k for k, v in net.items() if v > 0])
    return inputs, outputs


def items_touched_total(reactions: list[Reaction]) -> set[str]:
    s: set[str] = set()
    for r in reactions:
        s |= r.items_touched()
    return s


def find_min_single_pool_count(
    all_reactions: list[Reaction], target_net: Counter, cap: int = 6
) -> int | None:
    """
    在所有单步反应中, 找到能精确产出 target_net 的最少普通池台数.
    每台普通池跑一个反应(每个反应可重复 k 次 = k 台).
    返回最小台数; 若在枚举上限(cap)内无可行组合返回 None.

    实现: 把 target_net 规范成不可变签名, 用 DFS 逐反应决定其重复次数.
    """
    if not target_net:
        return 0
    target_keys = set(target_net)
    relevant = [r for r in all_reactions if r.items_touched() & target_keys]
    if not relevant:
        return None

    def key(c: Counter) -> tuple:
        return tuple(sorted(c.items()))

    target_sig = key(target_net)
    best: list[int | None] = [None]

    def dfs(i: int, acc: Counter, count: int):
        if best[0] is not None and count >= best[0]:
            return
        if count > cap:
            return
        # 逐物品可行性剪枝: 任何物品的绝对累积量已超过目标绝对量 => 剪掉
        for k, v in acc.items():
            tv = target_net.get(k, 0)
            if abs(v) > abs(tv) + 10:  # 宽松阈值, 仅剪明显不可行
                return
        if key(acc) == target_sig:
            if best[0] is None or count < best[0]:
                best[0] = count
            return
        if i >= len(relevant):
            return
        r = relevant[i]
        # 该反应重复 0..cap-count 次
        max_k = cap - count
        cur = Counter(acc)
        for k in range(0, max_k + 1):
            dfs(i + 1, cur, count + k)
            cur.update(r.net)

    dfs(0, Counter(), 0)
    return best[0]


# ---------------------------------------------------------------------------
# 报告
# ---------------------------------------------------------------------------

def fmt_stack(net: Counter, keys: list[str], meta: dict) -> str:
    parts = []
    for k in keys:
        v = abs(net[k])
        zh = meta.get(k, {}).get("zh", k)
        fluid = "💧" if meta.get(k, {}).get("fluid") else "  "
        parts.append(f"{fluid}{zh}({k})×{v}")
    return ", ".join(parts) if parts else "—"


def main():
    parser = argparse.ArgumentParser(description="计算扩容反应池合并配方的省电情况")
    parser.add_argument("--recipes", type=Path, default=RECIPES_TOML)
    parser.add_argument("--items", type=Path, default=ITEMS_TOML)
    parser.add_argument("--slots", type=int, default=DEFAULT_SLOTS, help="扩容池物品槽位数")
    parser.add_argument("--max-reactions", type=int, default=DEFAULT_MAX_REACTIONS,
                        help="单台扩容池同时跑的最多不同反应数")
    parser.add_argument("--emit-toml", type=Path, default=None,
                        help="把省电的合并配方写成 mix_pool_2 的 toml 片段到该路径")
    parser.add_argument("--all", action="store_true",
                        help="列出全部候选, 包括无池内消化的凑数组合和省电为 0 的")
    args = parser.parse_args()

    # 强制 UTF-8 输出 (Windows 控制台默认 GBK)
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    reactions_raw = load_recipes(args.recipes)
    if not reactions_raw:
        print(f"未在 {args.recipes} 找到 mix_pool_1 反应", file=sys.stderr)
        return 1
    meta = load_item_meta(args.items)
    reactions = [Reaction(i, r) for i, r in enumerate(reactions_raw)]

    print("=" * 80)
    print("普通反应池(mix_pool_1)单步反应清单")
    print("=" * 80)
    for r in reactions:
        print(f"  R{r.idx}: {r.label()}")
    print(f"\n共 {len(reactions)} 条. 功率 {MIX_POOL_1_POWER}W/台, throughput {THROUGHPUT_PER_MIN}/min.")
    print(f"扩容池 {MIX_POOL_2_POWER}W/台, 物品槽位上限 {args.slots}, 最多 {args.max_reactions} 个不同反应.\n")

    candidates = []

    # 枚举所有 2..max_reactions 大小的不同反应组合
    for size in range(2, args.max_reactions + 1):
        for combo in combinations(reactions, size):
            touched = items_touched_total(combo)
            if len(touched) > args.slots:
                continue  # 超出槽位
            net = merged_net(list(combo))
            inputs, outputs = external_items(net)
            # 必须有外部输出才算有效配方
            if not outputs:
                continue
            # 内部消化 = 出现在反应中但净流量为 0 的物品
            internalized = sorted(touched - set(net))
            # 至少要消化一个中间产物, 否则只是把互不相关的反应塞一起(虽也省电, 但无内部消化)
            # 这里仍保留, 但标注 internalized 数量
            min_single = find_min_single_pool_count(reactions, net)
            single_power = (min_single or len(combo)) * MIX_POOL_1_POWER
            merged_power = MIX_POOL_2_POWER
            saving = single_power - merged_power
            candidates.append({
                "combo": list(combo),
                "net": net,
                "inputs": inputs,
                "outputs": outputs,
                "internalized": internalized,
                "slots_used": len(touched),
                "min_single": min_single,
                "single_power": single_power,
                "merged_power": merged_power,
                "saving": saving,
                "saving_pct": (saving / single_power * 100) if single_power else 0.0,
            })

    # 去重: 相同外部净效果只保留省电最多/槽位最少的一条
    def sig(c):
        return (tuple(c["inputs"]), tuple(c["outputs"]),
                tuple((k, c["net"][k]) for k in c["inputs"] + c["outputs"]))

    best_by_sig: dict = {}
    for c in candidates:
        s = sig(c)
        prev = best_by_sig.get(s)
        if prev is None or c["saving"] > prev["saving"] or (
            c["saving"] == prev["saving"] and c["slots_used"] < prev["slots_used"]
        ):
            best_by_sig[s] = c
    # 默认只保留「有池内消化中间产物」的候选: 合并的真正意义就是消化中间流体,
    # 否则只是把互不相关的反应硬塞一台机器凑省电, 在游戏里没有实际价值.
    require_internal = not args.all
    candidates = sorted(
        best_by_sig.values(),
        key=lambda c: (
            -(1 if c["internalized"] else 0),   # 有内部消化的优先
            -c["saving"],
            -c["saving_pct"],
            c["slots_used"],
        ),
    )

    shown = [c for c in candidates if c["saving"] > 0 or args.all]
    if require_internal:
        shown = [c for c in shown if c["internalized"]]

    print("=" * 80)
    print("合并配方省电分析" + (" (仅含池内消化中间产物的)" if require_internal else ""))
    print("=" * 80)
    if not shown:
        print("没有找到符合条件的合并配方. (尝试 --all 或调大 --max-reactions)")
        return 0

    # 汇总表
    print(f"\n{'#':>3}  {'反应':>4}  {'省电':>6}  {'占比':>6}  {'槽位':>4}  {'池内消化'}")
    print("-" * 80)
    for i, c in enumerate(shown, 1):
        combo_ids = "+".join(f"R{r.idx}" for r in c["combo"])
        int_lbl = ", ".join(
            meta.get(k, {}).get("zh", k) for k in c["internalized"]
        ) or "—"
        print(f"{i:>3}  {combo_ids:>4}  {c['saving']:>4}W  {c['saving_pct']:>5.1f}%  "
              f"{c['slots_used']:>2}/{args.slots}  {int_lbl}")

    for i, c in enumerate(shown, 1):
        combo = c["combo"]
        print("\n" + "-" * 80)
        print(f"\n[{i}] 合并 {len(combo)} 个反应 -> 扩容池 1 台 ({c['merged_power']}W)")
        for r in combo:
            print(f"      + R{r.idx}: {r.label()}")
        print(f"   外部输入 : {fmt_stack(c['net'], c['inputs'], meta)}")
        print(f"   外部输出 : {fmt_stack(c['net'], c['outputs'], meta)}")
        if c["internalized"]:
            int_lbl = ", ".join(
                f"{'💧' if meta.get(k, {}).get('fluid') else '  '}{meta.get(k, {}).get('zh', k)}({k})"
                for k in c["internalized"]
            )
            print(f"   池内消化 : {int_lbl}")
        print(f"   物品槽位 : {c['slots_used']}/{args.slots}")
        ms = c["min_single"]
        ms_str = f"{ms} 台 = {c['single_power']}W" if ms is not None else f"≈{len(combo)} 台 = {c['single_power']}W (未找到更优单池组合)"
        print(f"   普通池方案: {ms_str}")
        print(f"   扩容池方案: 1 台 = {c['merged_power']}W")
        if c["saving"] > 0:
            print(f"   ✅ 省电 {c['saving']}W ({c['saving_pct']:.1f}%)  省占地 {ms if ms else len(combo)} -> 1 台")
        elif c["saving"] == 0:
            print(f"   ➖ 平手 (不省电, 但可能作为 feeder 消化中间流体)")
        else:
            print(f"   ❌ 更费电 {-c['saving']}W")

    # 可选: 写出 toml
    if args.emit_toml:
        emit_toml([c for c in shown if c["saving"] > 0], args.emit_toml, meta)
        print(f"\n已写出省电合并配方到: {args.emit_toml}")

    return 0


def emit_toml(candidates: list[dict], path: Path, meta: dict):
    """把省电的合并配方写成 mix_pool_2 的 toml 片段."""
    buf = io.StringIO()
    buf.write("# Auto-generated by scripts/find_expanded_crucible_merges.py\n")
    buf.write("# 扩容反应池 (mix_pool_2) 合并配方: 多个单步反应在一台扩容池内同时进行,\n")
    buf.write("# 中间产物在池内消化. time_s 与单步一致(2s, 30/min), 不打折扣.\n")
    buf.write("# 省电来自用 1 台 100W 扩容池替代多台 50W 普通池.\n\n")
    for c in candidates:
        buf.write(f"# 合并 {len(c['combo'])} 反应, 省电 {c['saving']}W ({c['saving_pct']:.1f}%)\n")
        buf.write("[[recipes]]\n")
        buf.write('facility = "mix_pool_2"\n')
        buf.write("time_s = 2\n")
        buf.write('gasEnv = "None"\n')
        ings = [{"item": k, "count": abs(c["net"][k])} for k in c["inputs"]]
        prods = [{"item": k, "count": c["net"][k]} for k in c["outputs"]]
        buf.write("ingredients = " + toml_inline_list(ings) + "\n")
        buf.write("products = " + toml_inline_list(prods) + "\n\n")
    path.write_text(buf.getvalue(), encoding="utf-8")


def toml_inline_list(items: list[dict]) -> str:
    if not items:
        return "[]"
    inner = ", ".join(
        '{ item = "%s", count = %d }' % (it["item"], it["count"]) for it in items
    )
    return "[" + inner + "]"


if __name__ == "__main__":
    sys.exit(main())
