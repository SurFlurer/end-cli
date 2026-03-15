<script lang="ts">
  import IconActionButton from "../button/IconActionButton.svelte";
  import FlowEdge from "./FlowEdge.svelte";
  import { onMount } from "svelte";
  import Panel from "../pane/Panel.svelte";
  import PanelHeader from "../pane/PanelHeader.svelte";
  import {
    Background,
    Controls,
    MiniMap,
    SvelteFlow,
    type Edge,
    type EdgeTypes,
    type Node,
  } from "@xyflow/svelte";
  import type { Viewport } from "@xyflow/system";
  import type { HighlightEdgeLabelData } from "../../lib/graph/highlight-edge-label";
  import {
    buildFlowGraph,
    selectGraphHighlight,
    type BuildFlowGraphResult,
    type GraphHighlightSelection,
  } from "../../lib/graph/index";
  import type { FlowSnapshot } from "../../lib/export/flow-snapshot";
  import { translateByLang } from "../../lib/lang";
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

  interface EdgeFlowMeta {
    itemName: string;
    flowPerMin: number;
  }

  const edgeTypes: EdgeTypes = {
    default: FlowEdge,
  };
  const FLOW_SPLIT_EPSILON = 0.005;
  const UPSTREAM_EDGE_STYLE = "stroke:#ab6a31;stroke-width:2.4;opacity:1;";
  const DOWNSTREAM_EDGE_STYLE = "stroke:#1f6176;stroke-width:2.4;opacity:1;";
  const BOTH_EDGE_STYLE = "stroke:#376f79;stroke-width:2.4;opacity:1;";
  const INACTIVE_EDGE_STYLE = "opacity:0.14;";
  const UPSTREAM_LABEL_STYLE =
    "color:#7d512b;font-weight:700;white-space:pre-line;text-align:center;";
  const DOWNSTREAM_LABEL_STYLE =
    "color:#154a59;font-weight:700;white-space:pre-line;text-align:center;";
  const BOTH_LABEL_STYLE =
    "color:#285963;font-weight:700;white-space:pre-line;text-align:center;";
  const INACTIVE_LABEL_STYLE = "color:#7a95a2;opacity:0.14;";

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
  let baseEdgeLabelById = new Map<string, Edge["label"]>();
  let baseEdgeLabelStyleById = new Map<string, string | undefined>();
  let baseEdgeDataById = new Map<string, Edge["data"]>();
  let edgeFlowMetaById = new Map<string, EdgeFlowMeta>();

  export function getFlowSnapshot(): FlowSnapshot | null {
    if (!result) {
      return null;
    }

    return { nodes, edges };
  }

  function appendStyle(
    baseStyle: string | undefined,
    extension: string,
  ): string {
    return `${baseStyle ?? ""}${extension}`;
  }

  function formatPerMin(value: number): string {
    return `${value.toFixed(2)}/min`;
  }

  function buildLabelData(
    meta: EdgeFlowMeta,
    usedPerMin: number,
    mutedPerMin?: number,
  ): HighlightEdgeLabelData {
    return {
      kind: "flow-highlight-label",
      topLine: meta.itemName,
      mainRateText: formatPerMin(usedPerMin),
      mutedRateText:
        mutedPerMin !== undefined && mutedPerMin > FLOW_SPLIT_EPSILON
          ? ` + ${formatPerMin(mutedPerMin)}`
          : undefined,
    };
  }

  function flattenLabelData(labelData: HighlightEdgeLabelData): string {
    return `${labelData.topLine}\n${labelData.mainRateText}${labelData.mutedRateText ?? ""}`;
  }

  function applyHighlightSelection(
    selection: GraphHighlightSelection | null,
  ): void {
    if (!selection) {
      nodes = nodes.map((node) => ({
        ...node,
        style: baseNodeStyleById.get(node.id),
      }));
      edges = edges.map((edge) => ({
        ...edge,
        data: baseEdgeDataById.get(edge.id),
        label: baseEdgeLabelById.get(edge.id),
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
      const isUpstream = selection.upstreamEdgeIds.has(edge.id);
      const isDownstream = selection.downstreamEdgeIds.has(edge.id);
      const edgeFlowMeta = edgeFlowMetaById.get(edge.id);

      let edgeData = baseEdgeDataById.get(edge.id);
      let edgeLabel = baseEdgeLabelById.get(edge.id);
      if (isHighlighted && edgeFlowMeta) {
        if (isUpstream) {
          const usedPerMin = Math.min(
            edgeFlowMeta.flowPerMin,
            Math.max(
              0,
              selection.upstreamUsedPerMinByEdgeId.get(edge.id) ??
                edgeFlowMeta.flowPerMin,
            ),
          );
          const unusedPerMin = Math.max(
            0,
            edgeFlowMeta.flowPerMin - usedPerMin,
          );
          const labelData = buildLabelData(
            edgeFlowMeta,
            usedPerMin,
            unusedPerMin,
          );
          edgeData = labelData as Edge["data"];
          edgeLabel = flattenLabelData(labelData);
        } else {
          const labelData = buildLabelData(
            edgeFlowMeta,
            edgeFlowMeta.flowPerMin,
          );
          edgeData = labelData as Edge["data"];
          edgeLabel = flattenLabelData(labelData);
        }
      }

      let highlightEdgeStyle = BOTH_EDGE_STYLE;
      let highlightLabelStyle = BOTH_LABEL_STYLE;
      if (isUpstream && !isDownstream) {
        highlightEdgeStyle = UPSTREAM_EDGE_STYLE;
        highlightLabelStyle = UPSTREAM_LABEL_STYLE;
      } else if (!isUpstream && isDownstream) {
        highlightEdgeStyle = DOWNSTREAM_EDGE_STYLE;
        highlightLabelStyle = DOWNSTREAM_LABEL_STYLE;
      }

      return {
        ...edge,
        data: edgeData,
        label: edgeLabel,
        style: appendStyle(
          baseEdgeStyleById.get(edge.id),
          isHighlighted ? highlightEdgeStyle : INACTIVE_EDGE_STYLE,
        ),
        labelStyle: appendStyle(
          baseEdgeLabelStyleById.get(edge.id),
          isHighlighted ? highlightLabelStyle : INACTIVE_LABEL_STYLE,
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
      baseEdgeLabelById = new Map();
      baseEdgeLabelStyleById = new Map();
      baseEdgeDataById = new Map();
      edgeFlowMetaById = new Map();
      nodes = [];
      edges = [];
      viewport = { x: 0, y: 0, zoom: 1 };
      return;
    }

    // Reset graph elements when we get a fresh solve result.
    highlightedNodeId = null;
    nodes = flow.nodes;
    edges = flow.edges;
    baseNodeStyleById = new Map(
      flow.nodes.map((node: Node) => [node.id, node.style]),
    );
    baseEdgeStyleById = new Map(
      flow.edges.map((edge: Edge) => [edge.id, edge.style]),
    );
    baseEdgeLabelById = new Map(
      flow.edges.map((edge: Edge) => [edge.id, edge.label]),
    );
    baseEdgeLabelStyleById = new Map(
      flow.edges.map((edge: Edge) => [edge.id, edge.labelStyle]),
    );
    baseEdgeDataById = new Map(
      flow.edges.map((edge: Edge) => [edge.id, edge.data]),
    );
    edgeFlowMetaById = new Map(
      result.logisticsGraph.edges.map((edge) => [
        edge.id,
        {
          itemName: edge.itemName,
          flowPerMin: edge.flowPerMin,
        } satisfies EdgeFlowMeta,
      ]),
    );
  });

  function t(zh: string, en: string): string {
    return translateByLang(lang, zh, en);
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
      document.removeEventListener(
        "webkitfullscreenchange",
        onFullscreenChange,
      );
    };
  });
</script>

<Panel contentMode="flush">
  {#snippet header()}
    <PanelHeader
      titleText={t("产线可视化", "Flow Map")}
      icon="flowchart"
      fieldHintText={t(
        "点击节点聚焦上下游，节点代表机器和输入输出，线条表示物品流",
        "Click a node to focus on its upstream and downstream. Nodes represent machines and inputs/outputs, and lines indicate item flow.",
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
        {edgeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
        onnodeclick={handleNodeClick}
        onpaneclick={handlePaneClick}
      >
        <Background
          bgColor="var(--surface-graph)"
          patternColor="var(--surface-graph-grid)"
          gap={24}
        />
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

  :global(.svelte-flow__pane.draggable) {
    cursor: default !important;
  }
  :global(.svelte-flow__pane.dragging) {
    cursor: default !important;
  }
</style>
