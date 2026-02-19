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
  }

  let { lang, result }: Props = $props();
  let flowElement = $state<HTMLElement | null>(null);
  let isFullscreen = $state(false);

  const flow = $derived<{ nodes: Node[]; edges: Edge[] }>(
    result ? buildFlowGraph(result.logisticsGraph) : { nodes: [], edges: [] },
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
    const fsElement = getFullscreenElement();
    if (fsElement === null || flowElement === null) {
      isFullscreen = false;
    } else {
      isFullscreen = flowElement.contains(fsElement as globalThis.Node);
    }
  }

  async function toggleFullscreen(): Promise<void> {
    if (!flowElement || typeof document === "undefined") {
      return;
    }

    try {
      const doc = document as FullscreenDocument;
      const element = flowElement as FullscreenHostElement;

      if (getFullscreenElement() === flowElement) {
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

<section class="graph-panel">
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
      <button
        type="button"
        class="fullscreen-toggle"
        aria-label={t("全屏", "Fullscreen")}
        title={t("全屏", "Fullscreen")}
        onclick={() => {
          void toggleFullscreen();
        }}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true">
          fullscreen
        </span>
      </button>
    {/if}
  </div>

  {#if !result}
    <p class="hint">
      {t("求解后这里显示物流网络。", "Logistics network appears after solving.")}
    </p>
  {:else}
    <div class="flow-wrap" bind:this={flowElement}>
      <SvelteFlow nodes={flow.nodes} edges={flow.edges} fitView>
        <Background bgColor="#f9fcfa" patternColor="#d8e6de" gap={24} />
        <MiniMap pannable zoomable />
        <Controls />
      </SvelteFlow>
      {#if isFullscreen}
        <button
          type="button"
          class="exit-fullscreen-float"
          aria-label={t("退出全屏", "Exit fullscreen")}
          title={t("退出全屏", "Exit fullscreen")}
          onclick={() => {
            void toggleFullscreen();
          }}
        >
          <span class="material-symbols-outlined icon" aria-hidden="true">
            fullscreen_exit
          </span>
        </button>
      {/if}
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
    position: relative;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    height: clamp(380px, 52vh, 720px);
    overflow: hidden;
    background: var(--panel-strong);
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

  .exit-fullscreen-float {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 10;
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
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .exit-fullscreen-float:hover {
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

  .flow-wrap:fullscreen,
  .flow-wrap:-webkit-full-screen {
    width: 100%;
    height: 100%;
    border-radius: 0;
  }

  @media (max-width: 760px) {
    .flow-wrap {
      height: clamp(300px, 56vh, 560px);
    }
  }
</style>
