<script lang="ts">
  import CopyButton from "./CopyButton.svelte";
  import DataTable from "./DataTable.svelte";
  import { onDestroy } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import Panel from "./Panel.svelte";
  import StatusPill, {
    type SolveStatusPillState,
  } from "./StatusPill.svelte";
  import type { LogisticsGraphDto } from "../lib/types";
  import type { LangTag } from "../lib/types";
  import {
    errorMessageOf,
    isSolveBusy,
    renderedOkState,
    type SolveState,
  } from "../lib/solve-state";

  interface Props {
    lang: LangTag;
    isBootstrapping: boolean;
    solveState: SolveState;
  }

  let { lang, isBootstrapping, solveState }: Props = $props();

  let liveElapsedMs = $state<number | null>(null);
  let solveTimerId: number | null = null;

  const isSolving = $derived(isSolveBusy(solveState));
  const renderedOk = $derived(renderedOkState(solveState));
  const result = $derived(renderedOk?.payload ?? null);
  const errorMessage = $derived(errorMessageOf(solveState));
  const showError = $derived(errorMessage.trim().length > 0);
  const headerElapsedMs = $derived.by((): number | null => {
    if (solveState.status === "solving") {
      return liveElapsedMs;
    }

    if (solveState.status === "ok") {
      return solveState.elapsedMs;
    }

    return null;
  });

  const solveMetaState = $derived.by((): SolveStatusPillState => {
    if (isSolving) {
      return { status: "solving", elapsedMs: headerElapsedMs };
    }

    if (showError) {
      return { status: "error", elapsedMs: headerElapsedMs };
    }

    return { status: "ok", elapsedMs: headerElapsedMs };
  });

  const solveOutputText = $derived.by(() => {
    if (errorMessage.trim().length > 0) {
      return errorMessage.trim();
    }

    if (result) {
      return JSON.stringify(result, null, 2);
    }

    return "";
  });

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function computeStockpileKpi(graph: LogisticsGraphDto): {
    itemKindCount: number;
    totalPerMin: number;
  } {
    const warehouseNode = graph.nodes.find(
      (n) => n.kind === "warehouse_stockpile",
    );
    if (!warehouseNode) {
      return { itemKindCount: 0, totalPerMin: 0 };
    }

    const itemKeys = new SvelteSet<string>();
    let totalPerMin = 0;
    for (const edge of graph.edges) {
      if (edge.target !== warehouseNode.id) {
        continue;
      }
      if (edge.flowPerMin <= 0) {
        continue;
      }
      itemKeys.add(edge.itemKey);
      totalPerMin += edge.flowPerMin;
    }

    return { itemKindCount: itemKeys.size, totalPerMin };
  }

  function stopSolveTimer(): void {
    if (solveTimerId === null || typeof window === "undefined") {
      return;
    }

    window.clearInterval(solveTimerId);
    solveTimerId = null;
  }

  function tickLiveElapsed(startedAt: number): void {
    liveElapsedMs = Math.max(0, Math.round(performance.now() - startedAt));
  }

  $effect(() => {
    if (solveState.status !== "solving") {
      stopSolveTimer();
      liveElapsedMs = null;
      return;
    }

    const startedAt = solveState.startedAt;
    tickLiveElapsed(startedAt);

    if (solveTimerId === null && typeof window !== "undefined") {
      solveTimerId = window.setInterval(() => tickLiveElapsed(startedAt), 80);
    }

    return () => stopSolveTimer();
  });

  onDestroy(() => {
    stopSolveTimer();
  });
</script>

<Panel>
  {#snippet header()}
    <div class="panel-head">
      <div>
        <div class="panel-title-row">
          <h2>{t("方案评估", "Plan Summary")}</h2>
        </div>
        <p class="subtitle">
          {t(
            "每次编辑后自动刷新收益、电力平衡和产线规模。",
            "After each edit, this panel auto-updates revenue, power balance, and line size.",
          )}
        </p>
      </div>

      <div class="header-controls">
        <CopyButton {lang} text={solveOutputText} />
        <StatusPill {lang} state={solveMetaState} />
      </div>
    </div>
  {/snippet}

  {#if showError}
    <p class="error-message">{errorMessage}</p>
  {/if}

  {#if isBootstrapping}
    <p class="hint">
      {t("正在加载 wasm 与基础数据...", "Loading wasm and baseline data...")}
    </p>
  {:else if !result}
    <p class="hint">
      {t(
        "先在左侧修改任意参数，这里会自动更新结果。",
        "Edit any parameter on the left, and results will update here automatically.",
      )}
    </p>
  {:else}
    {@const stockpile = computeStockpileKpi(result.logisticsGraph)}

    <div class="kpi-grid">
      <article>
        <h3>{t("收益 / min", "Revenue / min")}</h3>
        <p>{result.summary.stage2RevenuePerMin.toFixed(2)}</p>
      </article>
      <article>
        <h3>{t("收益 / h", "Revenue / h")}</h3>
        <p>{result.summary.stage2RevenuePerHour.toFixed(0)}</p>
      </article>
      <article>
        <h3>{t("机器/热能池", "Machines/Thermal")}</h3>
        <p>{result.summary.totalMachines}/{result.summary.totalThermalBanks}</p>
      </article>
      <article>
        <h3>{t("囤货种类/总数/min", "Stockpiled kinds/total/min")}</h3>
        <p>{stockpile.itemKindCount}/{stockpile.totalPerMin.toFixed(2)}</p>
      </article>
      <article>
        <h3>{t("用电/发电", "Power Use/Gen")}</h3>
        <p>{result.summary.powerUseW}/{result.summary.powerGenW}</p>
      </article>
      <article>
        <h3>{t("电力余量", "Power Margin")}</h3>
        <p>{result.summary.powerMarginW} W</p>
      </article>
    </div>

    <DataTable
      title={t("据点收益", "Outpost Revenue")}
      headers={[
        t("据点", "Outpost"),
        t("触顶", "Cap"),
        t("收益/min", "Value/min"),
        t("上限/min", "Cap/min"),
      ]}
      rows={result.summary.outposts.map((outpost) => [
        outpost.name,
        outpost.capPerMin > 0 && outpost.ratio >= 0.9999
          ? {
              text: t("触顶", "Capped"),
              icon: "check_circle",
              className: "cell-good",
            }
          : {
              text: t("未触顶", "Not capped"),
              icon: "warning",
              className: "cell-warn",
              tooltip: t(
                "交易额无法触顶，可考虑增加更多原料供应",
                "Revenue cannot reach cap; consider adding more raw supply",
              ),
            },
        outpost.valuePerMin.toFixed(2),
        outpost.capPerMin.toFixed(2),
      ])}
      numericColumns={[2, 3]}
    />

    <DataTable
      title={t("外部供给利用率", "External Supply Utilization")}
      headers={[
        t("物品", "Item"),
        t("供给/min", "Supply/min"),
        t("使用/min", "Used/min"),
        t("利用率", "Utilization"),
      ]}
      rows={result.summary.externalSupplySlack
        .slice()
        .sort((a, b) => a.itemKey.localeCompare(b.itemKey))
        .map((row) => {
          const used = Math.max(0, row.supplyPerMin - row.slackPerMin);
          const ratio = row.supplyPerMin <= 0 ? 0 : used / row.supplyPerMin;
          return [
            row.itemName,
            row.supplyPerMin.toFixed(2),
            used.toFixed(2),
            `${(ratio * 100).toFixed(1)}%`,
          ];
        })}
      numericColumns={[1, 2, 3]}
    />

    <DataTable
      title={t("销售物品", "Sold Items")}
      headers={[
        t("据点", "Outpost"),
        t("物品", "Item"),
        t("数量/min", "Qty/min"),
        t("收益/min", "Value/min"),
      ]}
      rows={result.summary.topSales.map((sale) => [
        sale.outpostName,
        sale.itemName,
        sale.qtyPerMin.toFixed(2),
        sale.valuePerMin.toFixed(2),
      ])}
      numericColumns={[2, 3]}
    />

    <DataTable
      title={t("设施负载", "Facility Load")}
      headers={[
        t("设施", "Facility"),
        t("机器数", "Machines"),
        t("每台耗电", "Power/Unit"),
        t("总耗电", "Total Power"),
      ]}
      rows={result.summary.facilities
        .slice(0, 16)
        .map((facility) => [
          facility.name,
          `${facility.machines}`,
          `${facility.powerW} W`,
          `${facility.totalPowerW} W`,
        ])}
      numericColumns={[1, 2, 3]}
    />
  {/if}
</Panel>

<style>
  .panel-title-row {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .error-message {
    margin: 0;
    color: var(--danger);
    font-size: 14px;
    font-weight: 600;
  }

  .kpi-grid {
    display: grid;
    gap: var(--space-3);
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .kpi-grid article {
    border-radius: var(--radius-md);
    background: var(--panel-strong);
    border: 1px solid var(--line);
    padding: var(--space-3);
  }

  @media (hover: hover) and (pointer: fine) {
    .kpi-grid article:hover {
      background: var(--surface-soft);
    }
  }

  .kpi-grid h3 {
    font-size: 12px;
    color: var(--ink-soft);
    margin-bottom: 8px;
  }

  .kpi-grid p {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .hint {
    margin: 0;
    color: var(--muted-text);
  }
</style>
