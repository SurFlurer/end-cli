<script lang="ts">
  import {
    BaseEdge,
    EdgeLabel,
    getBezierPath,
    type EdgeProps,
  } from "@xyflow/svelte";
  import {
    isHighlightEdgeLabelData,
    type HighlightEdgeLabelData,
  } from "../../lib/graph/highlight-edge-label";

  let {
    interactionWidth,
    label,
    labelStyle,
    markerEnd,
    markerStart,
    sourcePosition,
    sourceX,
    sourceY,
    style,
    targetPosition,
    targetX,
    targetY,
    data,
  }: EdgeProps = $props();

  let [path, labelX, labelY] = $derived(
    getBezierPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
    }),
  );

  let structuredLabel = $derived<HighlightEdgeLabelData | null>(
    isHighlightEdgeLabelData(data) ? data : null,
  );
</script>

<BaseEdge
  {path}
  {markerStart}
  {markerEnd}
  {interactionWidth}
  {style}
/>

{#if structuredLabel}
  <EdgeLabel x={labelX} y={labelY} style={labelStyle} selectEdgeOnClick>
    <div class="flow-edge-label">
      <div class="top-line">{structuredLabel.topLine}</div>
      <div class="rate-line">
        <span>{structuredLabel.mainRateText}</span>
        {#if structuredLabel.mutedRateText}
          <span class="muted-rate">{structuredLabel.mutedRateText}</span>
        {/if}
      </div>
    </div>
  </EdgeLabel>
{:else if label}
  <EdgeLabel x={labelX} y={labelY} style={labelStyle} selectEdgeOnClick>
    <span class="fallback-label">{label}</span>
  </EdgeLabel>
{/if}

<style>
  .flow-edge-label {
    display: flex;
    flex-direction: column;
    gap: 1px;
    line-height: 1.15;
    white-space: pre;
  }

  .top-line {
    font-weight: 700;
  }

  .rate-line {
    white-space: pre;
  }

  .muted-rate {
    color: #8fa1aa;
  }

  .fallback-label {
    white-space: pre-line;
  }
</style>
