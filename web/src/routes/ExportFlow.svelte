<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    Background,
    SvelteFlow,
    type Edge,
    type Node,
  } from "@xyflow/svelte";
  import { buildFlowGraph } from "../lib/graph";
  import { decodeTomlFromShareParam } from "../lib/share-link";
  import type { LangTag, SolvePayload } from "../lib/types";
  import { solveScenario } from "../lib/wasm";

  type ExportReadyMessage = {
    kind: "end2.export.ready";
    token: string;
  };

  type ExportErrMessage = {
    kind: "end2.export.err";
    token: string;
    message: string;
  };

  function postToParent(message: ExportReadyMessage | ExportErrMessage): void {
    if (typeof window === "undefined") {
      return;
    }

    try {
      window.parent?.postMessage(message, window.location.origin);
    } catch {
      // ignore
    }
  }

  function parseLangTag(raw: string | null): LangTag {
    return raw === "en" ? "en" : "zh";
  }

  function parseSize(raw: string | null, fallback: number): number {
    const parsed = raw ? Number.parseInt(raw, 10) : NaN;
    if (!Number.isFinite(parsed)) {
      return fallback;
    }
    return Math.max(240, Math.min(8192, parsed));
  }

  function nextFrame(): Promise<void> {
    return new Promise((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }

  let token = $state("");
  let lang = $state<LangTag>("zh");
  let widthPx = $state(1600);
  let heightPx = $state(900);

  let payload = $state<SolvePayload | null>(null);
  let errorText = $state("");

  const flow = $derived<{ nodes: Node[]; edges: Edge[] }>(
    payload ? buildFlowGraph(payload.logisticsGraph) : { nodes: [], edges: [] },
  );

  onMount(() => {
    if (typeof window === "undefined") {
      return;
    }

    const url = new URL(window.location.href);
    token = url.searchParams.get("token") ?? "";
    lang = parseLangTag(url.searchParams.get("lang"));
    widthPx = parseSize(url.searchParams.get("w"), 1600);
    heightPx = parseSize(url.searchParams.get("h"), 900);

    const shareParam = url.searchParams.get("s") ?? "";

    const run = async () => {
      try {
        if (shareParam.trim().length === 0) {
          throw new Error("missing share param");
        }

        const tomlText = await decodeTomlFromShareParam(shareParam);
        const solved = await solveScenario(lang, tomlText);
        payload = solved;

        // Give SvelteFlow a couple of turns to mount & fitView.
        await tick();
        await nextFrame();
        await tick();

        postToParent({ kind: "end2.export.ready", token });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        errorText = message;
        postToParent({
          kind: "end2.export.err",
          token,
          message,
        });
      }
    };

    void run();
  });
</script>

<div class="export-root" style={`--export-w: ${widthPx}px; --export-h: ${heightPx}px;`}>
  {#if payload}
    <div class="flow-wrap" id="logistics-flow-map">
      <SvelteFlow
        nodes={flow.nodes}
        edges={flow.edges}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background
          bgColor="var(--surface-graph)"
          patternColor="var(--surface-graph-grid)"
          gap={24}
        />
      </SvelteFlow>
    </div>
  {:else}
    <div class="loading" aria-busy="true">
      <p class="hint">{errorText.trim().length > 0 ? errorText : "loading"}</p>
    </div>
  {/if}
</div>

<style>
  .export-root {
    width: var(--export-w);
    height: var(--export-h);
    overflow: hidden;
  }

  .loading {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--panel-strong);
  }

  .hint {
    margin: 0;
    color: var(--muted-text);
    font-size: 14px;
    line-height: 1.4;
    text-align: center;
    padding: var(--space-3);
  }

  .flow-wrap {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--panel-strong);
    --xy-edge-label-background-color: var(--surface-graph);
  }
</style>
