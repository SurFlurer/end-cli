<script lang="ts">
  import IconActionButton from "../button/IconActionButton.svelte";
  import { onMount } from "svelte";
  import Panel from "../pane/Panel.svelte";
  import PanelHeader from "../pane/PanelHeader.svelte";
  import {
    Background,
    Controls,
    MiniMap,
    SvelteFlow,
    type Edge,
    type Node,
  } from "@xyflow/svelte";
  import type { Viewport } from "@xyflow/system";
  import {
    buildFlowGraph,
    selectGraphHighlight,
    type BuildFlowGraphResult,
    type GraphHighlightSelection,
  } from "../../lib/graph";
  import { currentFlowSnapshot } from "../../lib/export/flow-snapshot";
  import type { LangTag } from "../../lib/types";
  import { renderedOkState, type SolveState } from "../../lib/solve-state";

  type FullscreenDocument = Document & {
    webkitFullscreenElement?: Element | null;
    webkitExitFullscreen?: () => void | Promise<void>;
  };

  type FullscreenHostElement = HTMLElement & {
    webkitRequestFullscreen?: () => void | Promise<void>;
  };

  interface Props {
    lang: LangTag;
    solveState: SolveState;
  }

  let { lang, solveState }: Props = $props();
  let flowElement = $state<HTMLElement | null>(null);
  let isFullscreen = $state(false);
  let highlightedNodeId = $state<string | null>(null);

  let result = $derived(renderedOkState(solveState)?.payload ?? null);

  const flow = $derived<BuildFlowGraphResult | null>(
    result ? buildFlowGraph(result.logisticsGraph) : null,
  );

  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let viewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 });
  let baseNodeStyleById = new Map<string, string | undefined>();
  let baseEdgeStyleById = new Map<string, string | undefined>();
  let baseEdgeLabelStyleById = new Map<string, string | undefined>();

  function appendStyle(baseStyle: string | undefined, extension: string): string {
    return `${baseStyle ?? ""}${extension}`;
  }

  function applyHighlightSelection(selection: GraphHighlightSelection | null): void {
    if (!selection) {
      nodes = nodes.map((node) => ({
        ...node,
        style: baseNodeStyleById.get(node.id),
      }));
      edges = edges.map((edge) => ({
        ...edge,
        style: baseEdgeStyleById.get(edge.id),
        labelStyle: baseEdgeLabelStyleById.get(edge.id),
      }));
      return;
    }

    nodes = nodes.map((node) => {
      const isHighlighted = selection.nodeIds.has(node.id);
      return {
        ...node,
        style: appendStyle(
          baseNodeStyleById.get(node.id),
          isHighlighted ? "opacity:1;" : "opacity:0.22;",
        ),
      };
    });

    edges = edges.map((edge) => {
      const isHighlighted = selection.edgeIds.has(edge.id);
      return {
        ...edge,
        style: appendStyle(
          baseEdgeStyleById.get(edge.id),
          isHighlighted
            ? "stroke:#1f6176;stroke-width:2.4;opacity:1;"
            : "opacity:0.14;",
        ),
        labelStyle: appendStyle(
          baseEdgeLabelStyleById.get(edge.id),
          isHighlighted ? "fill:#154a59;font-weight:700;" : "fill:#7a95a2;",
        ),
      };
    });
  }

  function clearHighlight(): void {
    highlightedNodeId = null;
    applyHighlightSelection(null);
  }

  function handleNodeClick({ node }: { node: Node }): void {
    if (!flow) {
      return;
    }

    if (highlightedNodeId === node.id) {
      clearHighlight();
      return;
    }

    highlightedNodeId = node.id;
    applyHighlightSelection(
      selectGraphHighlight(flow.highlightIndex, {
        startNodeId: node.id,
        direction: "both",
        sccTraversal: "collapsed",
      }),
    );
  }

  function handlePaneClick(): void {
    clearHighlight();
  }

  $effect(() => {
    if (!result || !flow) {
      highlightedNodeId = null;
      baseNodeStyleById = new Map();
      baseEdgeStyleById = new Map();
      baseEdgeLabelStyleById = new Map();
      nodes = [];
      edges = [];
      viewport = { x: 0, y: 0, zoom: 1 };
      currentFlowSnapshot.set(null);
      return;
    }

    // Reset graph elements when we get a fresh solve result.
    highlightedNodeId = null;
    nodes = flow.nodes;
    edges = flow.edges;
    baseNodeStyleById = new Map(flow.nodes.map((node) => [node.id, node.style]));
    baseEdgeStyleById = new Map(flow.edges.map((edge) => [edge.id, edge.style]));
    baseEdgeLabelStyleById = new Map(flow.edges.map((edge) => [edge.id, edge.labelStyle]));
  });

  $effect(() => {
    if (!result) {
      return;
    }

    // Keep an up-to-date snapshot for export/share.
    currentFlowSnapshot.set({ nodes, edges, viewport });
  });

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
    document.addEventListener("webkitfullscreenchange", onFullscreenChange);

    return () => {
      document.removeEventListener("fullscreenchange", onFullscreenChange);
      document.removeEventListener("webkitfullscreenchange", onFullscreenChange);
    };
  });
</script>

<Panel contentMode="flush">
  {#snippet header()}
    <PanelHeader
      titleText={t("物流图", "Flow Map")}
      subtitleText={t(
        "节点是机器和输入输出，线条表示物品流动。",
        "Nodes represent machines and inputs/outputs, and lines indicate item flow. ",
      )}
    >
      {#snippet controls()}
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
      {/snippet}
    </PanelHeader>
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
      <div class="flow-wrap" id="logistics-flow-map" bind:this={flowElement}>
        <SvelteFlow
          bind:nodes
          bind:edges
          bind:viewport
          fitView
          proOptions={{ hideAttribution: true }}
          onnodeclick={handleNodeClick}
          onpaneclick={handlePaneClick}
        >
          <Background bgColor="var(--surface-graph)" patternColor="var(--surface-graph-grid)" gap={24} />
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
    --xy-edge-label-background-color: var(--surface-graph);
  }

  :global(.exit-fullscreen-float) {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 10;
    box-shadow: var(--shadow-floating);
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
