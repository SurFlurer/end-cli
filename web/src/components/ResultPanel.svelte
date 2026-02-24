<script lang="ts">
  import DataTable from "./DataTable.svelte";
  import IconActionButton from "./IconActionButton.svelte";
  import { onDestroy } from "svelte";
  import Panel from "./Panel.svelte";
  import type { LangTag, SolvePayload } from "../lib/types";

  interface Props {
    lang: LangTag;
    isBootstrapping: boolean;
    isSolving: boolean;
    solveElapsedMs: number | null;
    result: SolvePayload | null;
    errorMessage: string;
  }

  type CopyState = "idle" | "copied" | "failed";

  let {
    lang,
    isBootstrapping,
    isSolving,
    solveElapsedMs,
    result,
    errorMessage,
  }: Props = $props();

  let liveElapsedMs = $state<number | null>(null);
  let solveStartedAt = $state<number | null>(null);
  let solveTimerId: number | null = null;

  let copyState = $state<CopyState>("idle");
  let copyStateTimerId: number | null = null;

  const headerElapsedMs = $derived<number | null>(
    isSolving ? liveElapsedMs : solveElapsedMs,
  );

  const showError = $derived(errorMessage.trim().length > 0);

  const solveOutputText = $derived.by(() => {
    if (errorMessage.trim().length > 0) {
      return errorMessage.trim();
    }

    if (result) {
      return JSON.stringify(result, null, 2);
    }

    return "";
  });

  const copyButtonLabel = $derived.by(() => {
    if (copyState === "copied") {
      return t("已复制", "Copied");
    }

    if (copyState === "failed") {
      return t("复制失败", "Copy failed");
    }

    return solveOutputText.length === 0
      ? t("暂无可复制内容", "Nothing to copy")
      : t("复制输出", "Copy output");
  });

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function formatElapsed(ms: number | null): string {
    if (ms === null) {
      return "--";
    }

    if (ms < 1000) {
      return `${ms} ms`;
    }

    const seconds = ms / 1000;
    return `${seconds.toFixed(seconds < 10 ? 2 : 1)} s`;
  }

  function stopSolveTimer(): void {
    if (solveTimerId === null || typeof window === "undefined") {
      return;
    }

    window.clearInterval(solveTimerId);
    solveTimerId = null;
  }

  function tickLiveElapsed(): void {
    if (solveStartedAt === null) {
      return;
    }

    liveElapsedMs = Math.max(0, Math.round(performance.now() - solveStartedAt));
  }

  function resetCopyStateLater(): void {
    if (copyStateTimerId !== null && typeof window !== "undefined") {
      window.clearTimeout(copyStateTimerId);
    }

    if (typeof window === "undefined") {
      return;
    }

    copyStateTimerId = window.setTimeout(() => {
      copyState = "idle";
      copyStateTimerId = null;
    }, 1400);
  }

  function fallbackCopy(text: string): boolean {
    if (typeof document === "undefined") {
      return false;
    }

    const input = document.createElement("textarea");
    input.value = text;
    input.setAttribute("readonly", "");
    input.style.position = "fixed";
    input.style.left = "-9999px";
    document.body.append(input);
    input.select();
    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch {
      copied = false;
    }
    input.remove();
    return copied;
  }

  async function copyOutput(): Promise<void> {
    if (solveOutputText.length === 0) {
      return;
    }

    let copied = false;
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      try {
        await navigator.clipboard.writeText(solveOutputText);
        copied = true;
      } catch {
        copied = false;
      }
    }

    if (!copied) {
      copied = fallbackCopy(solveOutputText);
    }

    copyState = copied ? "copied" : "failed";
    resetCopyStateLater();
  }

  $effect(() => {
    if (!isSolving) {
      stopSolveTimer();
      solveStartedAt = null;
      liveElapsedMs = solveElapsedMs;
      return;
    }

    if (solveStartedAt === null) {
      solveStartedAt = performance.now() - (solveElapsedMs ?? 0);
    }

    tickLiveElapsed();

    if (solveTimerId === null && typeof window !== "undefined") {
      solveTimerId = window.setInterval(tickLiveElapsed, 80);
    }

    return () => {
      stopSolveTimer();
    };
  });

  $effect(() => {
    solveOutputText;
    copyState = "idle";
  });

  onDestroy(() => {
    stopSolveTimer();

    if (copyStateTimerId !== null && typeof window !== "undefined") {
      window.clearTimeout(copyStateTimerId);
      copyStateTimerId = null;
    }
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
        <IconActionButton
          icon={copyState === "copied" ? "check" : "content_copy"}
          onClick={() => {
            void copyOutput();
          }}
          disabled={solveOutputText.length === 0}
          ariaLabel={copyButtonLabel}
          title={copyButtonLabel}
        />

        <div class="solve-meta" class:danger={showError}>
          {#if isSolving}
            <span class="spinner" aria-hidden="true"></span>
          {:else if errorMessage}
            <span
              class="material-symbols-outlined solve-icon danger"
              aria-hidden="true">error</span
            >
          {:else}
            <span class="material-symbols-outlined solve-icon" aria-hidden="true"
              >{solveElapsedMs === null ? "schedule" : "check_circle"}</span
            >
          {/if}
          <p class="elapsed" class:danger={showError}>
            {formatElapsed(headerElapsedMs)}
          </p>
        </div>
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
      <h3>{t("生产机器", "Machines")}</h3>
      <p>{result.summary.totalMachines}</p>
    </article>
    <article>
      <h3>{t("热能池", "Thermal Banks")}</h3>
      <p>{result.summary.totalThermalBanks}</p>
    </article>
    <article>
      <h3>{t("用电/发电", "Power Use/Gen")}</h3>
      <p>{result.summary.powerUseW}/{result.summary.powerGenW} W</p>
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
        t("收益/min", "Value/min"),
        t("上限/min", "Cap/min"),
        t("占比", "Ratio"),
      ]}
      rows={result.summary.outposts.map((outpost) => [
        outpost.name,
        outpost.valuePerMin.toFixed(2),
        outpost.capPerMin.toFixed(2),
        `${(outpost.ratio * 100).toFixed(1)}%`,
      ])}
      numericColumns={[1, 2, 3]}
    />

    <DataTable
      title={t("销售物品", "Sold Items")}
      headers={[t("物品", "Item"), t("据点", "Outpost"), t("收益/min", "Value/min")]}
      rows={result.summary.topSales.map((sale) => [
        sale.itemName,
        sale.outpostName,
        sale.valuePerMin.toFixed(2),
      ])}
      numericColumns={[2]}
    />

    <DataTable
      title={t("设施负载", "Facility Load")}
      headers={[
        t("设施", "Facility"),
        t("机器数", "Machines"),
        t("每台耗电", "Power/Unit"),
        t("总耗电", "Total Power"),
      ]}
      rows={result.summary.facilities.slice(0, 16).map((facility) => [
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

  .solve-meta {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    border-radius: 999px;
    background: color-mix(in srgb,var(--surface-soft) 60%,var(--accent-soft));
    padding: 8px 12px;
    min-height: var(--control-size);
  }

  .solve-meta.danger {
    background: #ffebe9;
  }

  @media (hover: hover) and (pointer: fine) {
    .solve-meta:hover {
      background: var(--accent-soft);
    }

    .solve-meta.danger:hover {
      background: #ffddda;
    }
  }

  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--line) 80%, #b5d0c5);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
    flex: 0 0 auto;
  }

  .solve-icon {
    font-size: 16px;
    line-height: 1;
    color: var(--accent);
    display: block;
    font-variation-settings:
      "FILL" 0,
      "wght" 600,
      "GRAD" 0,
      "opsz" 16;
  }

  .solve-icon.danger {
    color: var(--danger);
  }

  .elapsed {
    margin: 0;
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
    width: 50px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .elapsed.danger {
    color: var(--danger);
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

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  @media (max-width: 760px) {
    .kpi-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
