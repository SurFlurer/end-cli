<script lang="ts">
  import {
    Background,
    SvelteFlow,
    type Edge,
    type Node,
  } from "@xyflow/svelte";
  import type { Viewport } from "@xyflow/system";
  import FlowReadyNotifier from "./FlowReadyNotifier.svelte";

  interface Props {
    nodes: Node[];
    edges: Edge[];
    viewport: Viewport;
    width: number;
    height: number;
    onReady: () => void;
  }

  let { nodes, edges, viewport, width, height, onReady }: Props = $props();

  let renderNodes = $state<Node[]>([]);
  let renderEdges = $state<Edge[]>([]);
  let renderViewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 });

  $effect(() => {
    renderNodes = nodes;
    renderEdges = edges;
    renderViewport = viewport;
  });
</script>

<div class="export-root" style={`width: ${width}px; height: ${height}px;`}>
  <SvelteFlow
    id="export-flow"
    nodes={renderNodes}
    edges={renderEdges}
    viewport={renderViewport}
    width={width}
    height={height}
    proOptions={{ hideAttribution: true }}
    nodesDraggable={false}
    nodesConnectable={false}
    elementsSelectable={false}
    nodesFocusable={false}
    edgesFocusable={false}
    panOnDrag={false}
    panOnScroll={false}
    zoomOnScroll={false}
    zoomOnPinch={false}
    zoomOnDoubleClick={false}
    preventScrolling={true}
    onlyRenderVisibleElements={false}
    disableKeyboardA11y={true}
  >
    <FlowReadyNotifier onReady={onReady} />
    <Background
      bgColor="var(--surface-graph)"
      patternColor="var(--surface-graph-grid)"
      gap={24}
    />
  </SvelteFlow>
</div>

<style>
  .export-root {
    overflow: hidden;
  }
</style>
