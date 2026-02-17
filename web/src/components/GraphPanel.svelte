<script lang="ts">
  import { onMount } from "svelte";
  import {
    Background,
    Controls,
    MiniMap,
    SvelteFlow,
    type Edge,
    type Node,
  } from "@xyflow/svelte";
  import { buildFlowGraph } from "../lib/graph";
  import type { LangTag, SolvePayload } from "../lib/types";

  type FullscreenDocument = Document & {
    webkitFullscreenElement?: Element | null;
    webkitExitFullscreen?: () => void | Promise<void>;
  };

  type FullscreenHostElement = HTMLElement & {
    webkitRequestFullscreen?: () => void | Promise<void>;
  };

  interface Props {
    lang: LangTag;
    result: SolvePayload | null;
    graphFilter: "all" | string;
    onGraphFilterChange: (nextFilter: "all" | string) => void;
  }

  let { lang, result, graphFilter, onGraphFilterChange }: Props = $props();
  let panelElement = $state<HTMLElement | null>(null);
  let isFullscreen = $state(false);

  const flow = $derived<{ nodes: Node[]; edges: Edge[] }>(
    result
      ? buildFlowGraph(result.logisticsGraph, graphFilter)
      : { nodes: [], edges: [] },
  );

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function getFullscreenElement(): Element | null {
    if (typeof document === "undefined") {
      return null;
    }
    const doc = document as FullscreenDocument;
    return doc.fullscreenElement ?? doc.webkitFullscreenElement ?? null;
  }

  function syncFullscreenState(): void {
    isFullscreen = getFullscreenElement() === panelElement;
  }

  async function toggleFullscreen(): Promise<void> {
    if (!panelElement || typeof document === "undefined") {
      return;
    }

    try {
      const doc = document as FullscreenDocument;
      const element = panelElement as FullscreenHostElement;

      if (getFullscreenElement() === panelElement) {
        if (doc.exitFullscreen) {
          await doc.exitFullscreen();
        } else if (doc.webkitExitFullscreen) {
          await Promise.resolve(doc.webkitExitFullscreen());
        }
        return;
      }

      if (element.requestFullscreen) {
        await element.requestFullscreen();
      } else if (element.webkitRequestFullscreen) {
        await Promise.resolve(element.webkitRequestFullscreen());
      }
    } catch {
      syncFullscreenState();
    }
  }

  onMount(() => {
    syncFullscreenState();
    const onFullscreenChange = () => {
      syncFullscreenState();
    };

    document.addEventListener("fullscreenchange", onFullscreenChange);
    document.addEventListener(
      "webkitfullscreenchange",
      onFullscreenChange as EventListener,
    );

    return () => {
      document.removeEventListener("fullscreenchange", onFullscreenChange);
      document.removeEventListener(
        "webkitfullscreenchange",
        onFullscreenChange as EventListener,
      );
    };
  });
</script>

<section
  class={`graph-panel ${isFullscreen ? "is-fullscreen" : ""}`}
  bind:this={panelElement}
>
  <div class="sub-header">
    <div>
      <h2>{t("物流图", "Logistics Graph")}</h2>
      <p class="subtitle">
        {t(
          "用于定位产销链路和流量集中区域。",
          "Inspect flow concentration and route direction.",
        )}
      </p>
    </div>

    {#if result}
      <div class="header-controls">
        <select
          value={graphFilter}
          onchange={(event) => {
            onGraphFilterChange((event.currentTarget as HTMLSelectElement).value);
          }}
        >
          <option value="all">{t("全部物品", "All items")}</option>
          {#each result.logisticsGraph.items as item}
            <option value={item.itemKey}>{item.itemName}</option>
          {/each}
        </select>

        <button
          type="button"
          class="fullscreen-toggle"
          aria-pressed={isFullscreen}
          aria-label={isFullscreen
            ? t("退出全屏", "Exit fullscreen")
            : t("全屏", "Fullscreen")}
          title={isFullscreen
            ? t("退出全屏", "Exit fullscreen")
            : t("全屏", "Fullscreen")}
          onclick={() => {
            void toggleFullscreen();
          }}
        >
          <span class="material-symbols-outlined icon" aria-hidden="true">
            {isFullscreen ? "fullscreen_exit" : "fullscreen"}
          </span>
        </button>
      </div>
    {/if}
  </div>

  {#if !result}
    <p class="hint">
      {t("求解后这里显示物流网络。", "Logistics network appears after solving.")}
    </p>
  {:else}
    <div class="flow-wrap">
      <SvelteFlow nodes={flow.nodes} edges={flow.edges} fitView>
        <Background bgColor="#f9fcfa" patternColor="#d8e6de" gap={24} />
        <MiniMap pannable zoomable />
        <Controls />
      </SvelteFlow>
    </div>
  {/if}
</section>

<style>
  .graph-panel {
    display: grid;
    gap: var(--space-3);
    min-height: 0;
    align-content: start;
  }

  .flow-wrap {
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    height: clamp(380px, 52vh, 720px);
    overflow: hidden;
    background: var(--panel-strong);
  }

  select {
    border: 1px solid color-mix(in srgb, var(--line) 90%, #b7cec2);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    background: var(--panel-strong);
    color: inherit;
  }

  .fullscreen-toggle {
    border: 1px solid color-mix(in srgb, var(--line) 90%, #b7cec2);
    border-radius: var(--radius-sm);
    width: var(--control-size);
    height: var(--control-size);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-soft);
    color: inherit;
    padding: 0;
    cursor: pointer;
    line-height: 1;
  }

  .fullscreen-toggle:hover {
    background: color-mix(in srgb, var(--surface-soft) 60%, var(--accent-soft));
  }

  .icon {
    font-size: 20px;
    line-height: 1;
    display: block;
    font-variation-settings: "FILL" 0, "wght" 400, "GRAD" 0, "opsz" 20;
  }

  .hint {
    margin: 0;
    color: var(--muted-text);
  }

  .graph-panel:fullscreen,
  .graph-panel:-webkit-full-screen,
  .graph-panel.is-fullscreen {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: clamp(12px, 2vw, 24px);
    background: var(--panel);
    grid-template-rows: auto minmax(0, 1fr);
  }

  .graph-panel:fullscreen .flow-wrap,
  .graph-panel:-webkit-full-screen .flow-wrap,
  .graph-panel.is-fullscreen .flow-wrap {
    min-height: 0;
    height: 100%;
  }

  @media (max-width: 760px) {
    .flow-wrap {
      height: clamp(300px, 56vh, 560px);
    }
  }
</style>
