<script lang="ts">
  import Panel from "../components/Panel.svelte";
  import PanelHeader from "../components/PanelHeader.svelte";
  import type { LangTag } from "../lib/types";
  import "../styles/app-shell.css";

  function detectBrowserLang(): LangTag {
    if (typeof navigator === "undefined") {
      return "zh";
    }

    const preferred = Array.isArray(navigator.languages)
      ? [...navigator.languages, navigator.language]
      : [navigator.language];

    for (const tag of preferred) {
      const normalized = tag.trim().toLowerCase();
      if (normalized.startsWith("zh")) {
        return "zh";
      }
      if (normalized.startsWith("en")) {
        return "en";
      }
    }

    return "zh";
  }

  let lang = $state<LangTag>(detectBrowserLang());

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }
</script>

<div class="shell info-shell">
  <Panel>
    {#snippet header()}
      <PanelHeader
        titleText={t("这个网页如何工作", "How this web app works")}
        subtitleText={t(
          "基于两阶段 MILP：先最大化收益，再在最优收益解中优化次级目标。",
          "A two-stage MILP: maximize revenue first, then optimize a secondary objective among equally-revenue-optimal solutions.",
        )}
      >
        {#snippet controls()}
          <a class="nav-link" href="#/">{t("返回工具", "Back to App")}</a>
        {/snippet}
      </PanelHeader>
    {/snippet}

    <section class="content">
      <p>
        {t(
          "页面左侧的输入会被序列化成 aic.toml，然后交给浏览器中的 WASM 求解器（HiGHS）求解一个混合整数线性规划（MILP）。",
          "Inputs on the left are serialized into aic.toml, then solved by an in-browser WASM solver (HiGHS) as a Mixed-Integer Linear Program (MILP).",
        )}
      </p>

      <h3>{t("核心思路（简化版）", "Core idea (simplified)")}</h3>
      <ul>
        <li>
          {t(
            "用连续变量表示“每分钟跑多少次配方 / 卖多少货”。",
            "Use continuous variables for recipe rates (per minute) and sales flows.",
          )}
        </li>
        <li>
          {t(
            "用整数变量表示“要造多少台机器 / 热能池”。",
            "Use integer variables for how many machines / thermal banks are built.",
          )}
        </li>
        <li>
          {t(
            "用线性约束表达：物料守恒、据点交易额上限、机器吞吐限制、电力平衡等。",
            "Linear constraints enforce material balance, outpost money caps, machine throughput, and power balance.",
          )}
        </li>
        <li>
          {t(
            "目标函数分两阶段：Stage 1 先把收益推到最大；Stage 2 再在不降低真实收益的前提下优化你选择的次级目标。",
            "Two-stage objective: Stage 1 maximizes revenue; Stage 2 optimizes a chosen secondary objective without reducing real revenue.",
          )}
        </li>
      </ul>

      <h3>{t("（参考）变量与约束摘要", "(Reference) Variables & constraints")}</h3>
      <div class="mono-block" role="note" aria-label={t("模型摘要", "Model summary")}
        >
        <div class="mono-title">Stage 1 objective</div>
        <pre>maximize  Σ_o Σ_i  p[o,i] * q[o,i]</pre>

        <div class="mono-title">Material balance (per minute)</div>
        <pre>s[i] + Σ_r a[r,i] * x[r] - c[i] - Σ_o q[o,i]  ≥  0</pre>

        <div class="mono-title">Outpost money cap (per hour)</div>
        <pre>Σ_i p[o,i] * q[o,i]  ≤  C[o] / 60</pre>

        <div class="mono-title">Machine throughput</div>
        <pre>x[r]  ≤  Σ_t u[t,r] * Y[t,r]</pre>

        <div class="mono-title">Power balance</div>
        <pre>Σ_b P[b] * Z[b] + P_core  ≥  Σ_t w[t] * M[t] + P_ext</pre>

        <div class="mono-title">Stage 2</div>
        <pre>subject to  Σ_o Σ_i p[o,i] * q[o,i]  ≥  R*</pre>
        <pre>then optimize one of: min machines / max power slack / ...</pre>
      </div>

      <p class="note">
        {t(
          "上面是按文档 model_v1 的含义做的“阅读版”摘要；完整集合/参数/变量与 slack 规则见仓库文档。",
          "This is a readability-focused summary based on model_v1; see the repo docs for the full sets/params/variables and slack rules.",
        )}
      </p>
    </section>
  </Panel>
</div>

<style>
  .info-shell {
    height: auto;
    min-height: 100dvh;
  }

  .nav-link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    height: var(--control-size);
    border-radius: var(--radius-sm);
    background: var(--surface-soft);
    color: var(--accent-ink);
    text-decoration: none;
    font-weight: 600;
  }

  .nav-link:hover {
    background: var(--accent-soft);
  }

  .content {
    display: grid;
    gap: var(--space-3);
    line-height: 1.5;
  }

  ul {
    margin: 0;
    padding-left: 18px;
  }

  li {
    margin: var(--space-1) 0;
  }

  .mono-block {
    border: 1px solid color-mix(in srgb, var(--line) 70%, var(--line-tint-1));
    border-radius: var(--radius-md);
    background: var(--surface-soft);
    padding: var(--space-3);
    overflow: auto;
  }

  .mono-title {
    font-size: 12px;
    color: var(--muted-text);
    font-weight: 600;
    margin-top: var(--space-2);
  }

  .mono-title:first-child {
    margin-top: 0;
  }

  pre {
    margin: var(--space-1) 0 0;
    white-space: pre;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono",
      "Courier New", monospace;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink);
  }

  .note {
    color: var(--muted-text);
    font-size: 12px;
  }
</style>
