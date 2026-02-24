<script lang="ts">
  import IconActionButton from "./IconActionButton.svelte";
  import { onMount } from "svelte";
  import Panel from "./Panel.svelte";
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

<Panel contentMode="flush">
  {#snippet header()}
    <div class="sub-header">
      <div>
        <h2>{t("物流图", "Flow Map")}</h2>
        <p class="subtitle">
          {t(
            "节点是机器和输入输出，线条表示物品流动。",
            "Nodes represent machines and inputs/outputs, and lines indicate item flow. ",
          )}
        </p>
      </div>

      {#if result}
        <IconActionButton
          icon="fullscreen"
          ariaLabel={t("全屏", "Fullscreen")}
          title={t("全屏", "Fullscreen")}
          onClick={() => {
            void toggleFullscreen();
          }}
        />
      {/if}
    </div>
  {/snippet}

    {#if !result}
      <div class="hint-wrap">
        <p class="hint">
          {t(
            "先在左侧改一条参数并触发求解，随后这里会显示物流网络。",
            "Edit a parameter on the left to solve first, then the logistics network will appear here.",
          )}
        </p>
      </div>
    {:else}
      <div class="flow-wrap" bind:this={flowElement}>
        <SvelteFlow nodes={flow.nodes} edges={flow.edges} fitView proOptions={{ hideAttribution: true }}>
          <Background bgColor="#f9fcfa" patternColor="#d8e6de" gap={24} />
          {#if isFullscreen}
          <MiniMap pannable zoomable />
          <Controls />
          {/if}
        </SvelteFlow>
        {#if isFullscreen}
          <IconActionButton
            className="exit-fullscreen-float"
            icon="fullscreen_exit"
            ariaLabel={t("退出全屏", "Exit fullscreen")}
            title={t("退出全屏", "Exit fullscreen")}
            onClick={() => {
              void toggleFullscreen();
            }}
          />
        {/if}
      </div>
    {/if}
</Panel>

<style>
  .hint-wrap {
    padding: clamp(12px, 1.6vw, 16px);
  }

  .flow-wrap {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: var(--panel-strong);
    --xy-edge-label-background-color: #f9fcfa;
  }

  :global(.exit-fullscreen-float) {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 10;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
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
</style>
