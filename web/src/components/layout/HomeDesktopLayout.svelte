<script lang="ts">
  import DragImportOverlay from "../hover/DragImportOverlay.svelte";
  import EditorPanel from "../workbench/EditorPanel.svelte";
  import GraphPanel from "../workbench/GraphPanel.svelte";
  import HorizontalSplitter from "../pane/HorizontalSplitter.svelte";
  import ResultPanel from "../workbench/ResultPanel.svelte";
  import Splitter from "../pane/Splitter.svelte";
  import { onMount } from "svelte";
  import type { EditorActions } from "../../lib/editor-actions";
  import type { SolveState } from "../../lib/solve-state";
  import type { AicDraft, CatalogItemDto, LangTag } from "../../lib/types";
  import type { OutpostSelection } from "../../lib/outpost-selection";
  import {
    persistLeftPaneRatio,
    persistRightPaneRatio,
  } from "../../lib/draft-storage";

  const LEFT_PANE_RATIO_STORAGE_KEY = "end2.web.left-pane-ratio.v2";
  const RIGHT_PANE_RATIO_STORAGE_KEY = "end2.web.right-pane-ratio.v2";
  const DEFAULT_LEFT_PANE_RATIO = 0.55;
  const DEFAULT_RIGHT_PANE_RATIO = 0.5;
  const MIN_PANE_RATIO = 0.1;
  const MAX_PANE_RATIO = 0.9;

  interface Props {
    lang: LangTag;
    draft: AicDraft;
    catalogItems: CatalogItemDto[];
    selectedOutpostIndex: OutpostSelection;
    isBootstrapping: boolean;
    solveState: SolveState;
    editorActions: EditorActions;
    onOpenShare: () => void;
    onImportFile: (file: File) => void | Promise<void>;

    minEditorWidthPx: number;
    minRightWidthPx: number;
    minTopPanelHeightPx: number;
    minBottomPanelHeightPx: number;
  }

  let {
    lang,
    draft,
    catalogItems,
    selectedOutpostIndex,
    isBootstrapping,
    solveState,
    editorActions,
    onOpenShare,
    onImportFile,
    minEditorWidthPx,
    minRightWidthPx,
    minTopPanelHeightPx,
    minBottomPanelHeightPx,
  }: Props = $props();

  let leftPaneRatio = $state(DEFAULT_LEFT_PANE_RATIO);
  let rightPaneRatio = $state(DEFAULT_RIGHT_PANE_RATIO);
  let hasHydratedLocalState = $state(false);

  let layoutElement = $state<HTMLElement | null>(null);
  let rightPaneElement = $state<HTMLElement | null>(null);

  function getStorage(): Storage | null {
    if (typeof window === "undefined") {
      return null;
    }

    try {
      return window.localStorage;
    } catch {
      return null;
    }
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function parseRatio(raw: string | null): number | null {
    if (raw === null) {
      return null;
    }

    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      return null;
    }

    return clamp(parsed, MIN_PANE_RATIO, MAX_PANE_RATIO);
  }

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  onMount(() => {
    const storage = getStorage();
    if (storage) {
      const restoredLeft = parseRatio(storage.getItem(LEFT_PANE_RATIO_STORAGE_KEY));
      if (restoredLeft !== null) {
        leftPaneRatio = restoredLeft;
      }

      const restoredRight = parseRatio(storage.getItem(RIGHT_PANE_RATIO_STORAGE_KEY));
      if (restoredRight !== null) {
        rightPaneRatio = restoredRight;
      }
    }

    hasHydratedLocalState = true;
  });

  $effect(() => {
    if (!hasHydratedLocalState) {
      return;
    }

    persistLeftPaneRatio(LEFT_PANE_RATIO_STORAGE_KEY, leftPaneRatio);
  });

  $effect(() => {
    if (!hasHydratedLocalState) {
      return;
    }

    persistRightPaneRatio(RIGHT_PANE_RATIO_STORAGE_KEY, rightPaneRatio);
  });
</script>

<main
  class="workspace"
  bind:this={layoutElement}
  style={`--left-pane-width: ${(leftPaneRatio * 100).toFixed(2)}%`}
>
  <section class="editor">
    <EditorPanel
      {lang}
      {draft}
      {catalogItems}
      {selectedOutpostIndex}
      isResetDisabled={isBootstrapping}
      actions={editorActions}
      {onOpenShare}
    />
  </section>

  <Splitter
    {layoutElement}
    ratio={leftPaneRatio}
    minLeftPx={minEditorWidthPx}
    minRightPx={minRightWidthPx}
    ariaLabel={t("左右栏宽度调节", "Resize left and right columns")}
    onRatioChange={(nextRatio) => {
      leftPaneRatio = nextRatio;
    }}
  />

  <div
    class="right-pane"
    bind:this={rightPaneElement}
    style={`--right-top-height: ${(rightPaneRatio * 100).toFixed(2)}%; --right-min-top-height: ${minTopPanelHeightPx}px; --right-min-bottom-height: ${minBottomPanelHeightPx}px`}
  >
    <ResultPanel {lang} {isBootstrapping} {solveState} />

    <HorizontalSplitter
      layoutElement={rightPaneElement}
      ratio={rightPaneRatio}
      minTopPx={minTopPanelHeightPx}
      minBottomPx={minBottomPanelHeightPx}
      ariaLabel={t("上下栏高度调节", "Resize top and bottom panels")}
      onRatioChange={(nextRatio) => {
        rightPaneRatio = nextRatio;
      }}
    />

    <GraphPanel {lang} {solveState} />
  </div>

  <DragImportOverlay {lang} onImportFile={onImportFile} />
</main>
