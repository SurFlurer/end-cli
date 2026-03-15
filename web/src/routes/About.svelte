<script lang="ts">
  import Panel from "../components/pane/Panel.svelte";
  import PanelHeader from "../components/pane/PanelHeader.svelte";
  import { translateByLang } from "../lib/lang";
  import type { LangTag } from "../lib/types";

  const GITHUB_REPO_URL = "https://github.com/sssxks/end-cli";

  interface Props {
    lang: LangTag;
  }

  let { lang }: Props = $props();

  function t(zh: string, en: string): string {
    return translateByLang(lang, zh, en);
  }
</script>

<Panel>
  {#snippet header()}
    <PanelHeader
      titleText={t("关于这个 App", "About this app")}
      icon="info"
    ></PanelHeader>
  {/snippet}

  <section class="content">
    <h3>{t("能做什么", "What it does")}</h3>
    <p>
      {t(
        "这个 App 可以计算产线，然后变成漂亮的报告和可视化流程图，并且和陈千语、ChatGPT 分享。",
        "This app can compute production plans, then turn them into nice reports and visualized flow charts, and share with your friends and ChatGPT.",
      )}
    </p>
    <p>
      {t(
        "基于稳态模型，不考虑仓储容量和生产线布局因素，适合用来做高层次的规划和产线配平。",
        "Based on a steady-state model, it does not consider storage capacity and production line layout factors, suitable for high-level planning and production line balancing.",
      )}
    </p>
    <p>
      {t(
        "感兴趣可以看一下右边那一页的公式。",
        "If you're interested, you can check out the formulas on the right page.",
      )}
    </p>
    <h3>{t("怎么使用", "How to use it")}</h3>
    <p>
      {t(
        "首先，根据游戏进程在左侧输入当前矿点产量、据点价格与交易额上限、电力情况等信息。",
        "First, based on your game progress, enter on the left your current mine outputs, outpost prices and money caps, and power situation.",
      )}
    </p>
    <ol>
      <li>
        {t(
          "使用方式 1：已经明确知道想要生产的物品时，填写外部消耗里，据点列表可以留空，将本工具当作产线配平计算器使用。",
          "Mode 1: when you already know exactly which items you want to produce, put them into external consumption, leave the outpost list empty, and use this as a pure production-balancing calculator.",
        )}
      </li>
      <li>
        {t(
          "使用方式 2：需要探索各种生产方案时，填好据点收购价与交易额上限，让求解器在先打满据点收入的前提下，尽量优化剩余可支撑的物资生产和电力生产。",
          "Mode 2: when you want to explore different plans, fill in outpost buy prices and money caps so the solver first maxes out outpost income, then uses the remaining capacity to optimize additional goods and power production.",
        )}
      </li>
    </ol>
    <p>
      {t(
        "需要保存当前输入时，用导出按钮下载对应的 toml 文件，或者使用分享按钮生成一个包含输入信息的链接。",
        "When you want to save the current inputs, use Export to download the corresponding toml file, or use the Share button to generate a link containing the input information.",
      )}
    </p>
    <p>
      {t(
        "结果也可以导出成图片！",
        "The results can also be exported as images!",
      )}
    </p>

    <h3>{t("项目链接", "Links")}</h3>
    <ul>
      <li>
        <a href={GITHUB_REPO_URL} target="_blank" rel="noreferrer"
          >GitHub: sssxks/end-cli</a
        >
      </li>
    </ul>

    <p class="note">
      {t(
        "提需求、报错、讨论，建议直接来 GitHub 开 issue。",
        "For feature requests / bugs / modeling discussions, please open an issue on GitHub.",
      )}
    </p>
  </section>
</Panel>

<style>
  .content {
    display: grid;
    gap: var(--space-3);
    line-height: 1.5;

    max-width: 960px;
    margin: 0 auto;
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
