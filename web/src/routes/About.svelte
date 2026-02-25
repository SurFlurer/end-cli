<script lang="ts">
  import Panel from "../components/Panel.svelte";
  import PanelHeader from "../components/PanelHeader.svelte";
  import type { LangTag } from "../lib/types";
  import "../styles/app-shell.css";

  const GITHUB_REPO_URL = "https://github.com/sssxks/end-cli";
  const WEB_URL = "https://end-8jk.pages.dev/";

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
        titleText={t("关于这个工具", "About this tool")}
        subtitleText={t(
          "Rust + WebAssembly 的终末地产线规划器（MILP / HiGHS）。",
          "A Rust + WebAssembly production planner (MILP / HiGHS).",
        )}
      >
        {#snippet controls()}
          <a class="nav-link" href="#/">{t("返回工具", "Back to App")}</a>
        {/snippet}
      </PanelHeader>
    {/snippet}

    <section class="content">
      <h3>{t("它能做什么", "What it does")}</h3>
      <ul>
        <li>
          {t(
            "根据外部供给/消耗、据点价格与上限、外部耗电等输入，自动给出“卖什么、跑哪些配方、需要多少机器/热能池”的最优方案。",
            "Given external supply/consumption, outpost prices & caps, and external power usage, it computes an optimal plan (what to sell, which recipes to run, and how many machines/thermal banks).",
          )}
        </li>
        <li>
          {t(
            "求解在浏览器本地执行：WASM + WebWorker + HiGHS；导入/导出 aic.toml 也只发生在本地。",
            "Solving runs locally in your browser (WASM + WebWorker + HiGHS); importing/exporting aic.toml also stays local.",
          )}
        </li>
      </ul>

      <h3>{t("快速上手", "Quick start")}</h3>
      <ol>
        <li>
          {t(
            "在左侧“求解输入”里填写外部供给/外部消耗、据点收购价与交易额上限、外部耗电。",
            "Fill inputs on the left: supply/consumption, outpost prices & money caps, and external power.",
          )}
        </li>
        <li>
          {t(
            "右侧会自动求解并刷新“方案评估”和“物流图”。",
            "The right side auto-solves and updates the summary and flow graph.",
          )}
        </li>
        <li>
          {t(
            "需要保存时，用“导出”按钮下载当前输入的 aic.toml。",
            "Use Export to download the current aic.toml.",
          )}
        </li>
      </ol>

      <h3>{t("项目链接", "Links")}</h3>
      <ul>
        <li>
          <a href={GITHUB_REPO_URL} target="_blank" rel="noreferrer">GitHub: sssxks/end-cli</a>
        </li>
        <li>
          <a href={WEB_URL} target="_blank" rel="noreferrer">{t("网页版本", "Web app")}</a>
        </li>
      </ul>

      <p class="note">
        {t(
          "想提需求/报错/讨论模型细节：建议直接去 GitHub 开 issue。",
          "For feature requests / bugs / modeling discussions, please open an issue on GitHub.",
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

  ul,
  ol {
    margin: 0;
    padding-left: 18px;
    color: var(--ink);
  }

  li {
    margin: var(--space-1) 0;
  }

  a {
    color: var(--accent-ink);
  }

  .note {
    color: var(--muted-text);
    font-size: 12px;
  }
</style>
