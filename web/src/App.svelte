<script lang="ts">
  import { onMount } from "svelte";
  import EditorPanel from "./components/EditorPanel.svelte";
  import GraphPanel from "./components/GraphPanel.svelte";
  import ResultPanel from "./components/ResultPanel.svelte";
  import "./styles/app-shell.css";
  import { buildAicToml, parseAicToml } from "./lib/aic";
  import {
    addConsumptionRow,
    addOutpost,
    addPriceRow,
    addSupplyRow,
    normalizeSelectedOutpostIndex,
    removeConsumptionRow,
    removeOutpost,
    removePriceRow,
    removeSupplyRow,
    setConsumptionKey,
    setConsumptionValue,
    setExternalPower,
    setRegion,
    setOutpostField,
    setPriceKey,
    setPriceValue,
    setSupplyKey,
    setSupplyValue,
  } from "./lib/draft-actions";
  import {
    persistDraft,
    persistLeftPaneRatio,
    restoreLocalState,
    type DraftStorageConfig,
  } from "./lib/draft-storage";
  import type { EditorActions } from "./lib/editor-actions";
  import {
    createSolverController,
    type SolverController,
  } from "./lib/solver-controller";
  import type {
    AicDraft,
    CatalogItemDto,
    LangTag,
    SolvePayload,
  } from "./lib/types";
  import { EMPTY_DRAFT } from "./lib/types";
  import { loadBootstrap, solveScenario, warmupWasmWorker } from "./lib/wasm";

  const NARROW_LAYOUT_QUERY = "(max-width: 1160px)";
  const SPLITTER_WIDTH_PX = 12;
  const MIN_EDITOR_WIDTH_PX = 720;
  const MIN_RIGHT_WIDTH_PX = 420;
  const AUTO_SOLVE_DEBOUNCE_MS = 900;

  const STORAGE_CONFIG: DraftStorageConfig = {
    draftStorageKey: "end2.web.draft.v2",
    paneRatioStorageKey: "end2.web.left-pane-ratio.v2",
    minPaneRatio: 0.1,
    maxPaneRatio: 0.9,
  };

  function detectBrowserLang(): LangTag {
    if (typeof navigator === "undefined") {
      return "zh";
    }

    const preferred = Array.isArray(navigator.languages)
      ? [...navigator.languages, navigator.language]
      : [navigator.language];

    for (const tag of preferred) {
      const normalized = tag.trim().toLowerCase();
      if (normalized.startsWith("zh")) {
        return "zh";
      }
      if (normalized.startsWith("en")) {
        return "en";
      }
    }

    return "zh";
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  let lang = $state<LangTag>(detectBrowserLang());
  let catalogItems = $state<CatalogItemDto[]>([]);
  let draft = $state<AicDraft>(structuredClone(EMPTY_DRAFT));
  let defaultToml = $state("");

  let isBootstrapping = $state(true);
  let isSolving = $state(false);
  let solveElapsedMs = $state<number | null>(null);
  let errorMessage = $state("");

  let result = $state<SolvePayload | null>(null);
  let selectedOutpostIndex = $state(-1);

  let layoutElement = $state<HTMLElement | null>(null);
  let isNarrowScreen = $state(false);
  let activeTab = $state<"editor" | "result" | "graph">("editor");
  let leftPaneRatio = $state(0.55);
  let isDraggingSplitter = $state(false);
  let isDragImportActive = $state(false);
  let dragImportDepth = 0;

  let hasHydratedLocalState = $state(false);
  let hasRestoredDraftFromStorage = $state(false);

  let solverController: SolverController | null = null;

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function updatePaneRatioByClientX(clientX: number): void {
    if (!layoutElement) {
      return;
    }

    const rect = layoutElement.getBoundingClientRect();
    const usableWidth = rect.width - SPLITTER_WIDTH_PX;
    if (usableWidth <= 0) {
      return;
    }

    if (usableWidth <= MIN_EDITOR_WIDTH_PX + MIN_RIGHT_WIDTH_PX) {
      leftPaneRatio = 0.5;
      return;
    }

    const minRatio = MIN_EDITOR_WIDTH_PX / usableWidth;
    const maxRatio = 1 - MIN_RIGHT_WIDTH_PX / usableWidth;
    const nextRatio = (clientX - rect.left) / usableWidth;
    leftPaneRatio = clamp(nextRatio, minRatio, maxRatio);
  }

  function onSplitterPointerMove(event: PointerEvent): void {
    if (!isDraggingSplitter) {
      return;
    }
    updatePaneRatioByClientX(event.clientX);
  }

  function stopSplitResize(): void {
    if (!isDraggingSplitter) {
      return;
    }

    isDraggingSplitter = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    window.removeEventListener("pointermove", onSplitterPointerMove);
    window.removeEventListener("pointerup", stopSplitResize);
    window.removeEventListener("pointercancel", stopSplitResize);
  }

  function startSplitResize(event: PointerEvent): void {
    if (isNarrowScreen) {
      return;
    }

    event.preventDefault();
    isDraggingSplitter = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    updatePaneRatioByClientX(event.clientX);
    window.addEventListener("pointermove", onSplitterPointerMove);
    window.addEventListener("pointerup", stopSplitResize);
    window.addEventListener("pointercancel", stopSplitResize);
  }

  function applyToml(text: string): void {
    draft = parseAicToml(text);
    selectedOutpostIndex = draft.outposts.length > 0 ? 0 : -1;
    result = null;
    errorMessage = "";
    solverController?.resetSolvedFingerprint();
  }

  function isTomlFile(file: File): boolean {
    return file.name.trim().toLowerCase().endsWith(".toml");
  }

  async function importTomlFile(file: File): Promise<void> {
    if (!isTomlFile(file)) {
      errorMessage = t(
        "仅支持导入 .toml 文件（例如 aic.toml）。",
        "Only .toml files are supported for import (for example aic.toml).",
      );
      return;
    }

    try {
      const text = await file.text();
      applyToml(text);
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    }
  }

  function hasFileTransfer(event: DragEvent): boolean {
    const types = event.dataTransfer?.types;
    if (!types) {
      return false;
    }
    return Array.from(types).includes("Files");
  }

  function clearDragImportState(): void {
    dragImportDepth = 0;
    isDragImportActive = false;
  }

  function onWindowDragEnter(event: DragEvent): void {
    if (!hasFileTransfer(event)) {
      return;
    }

    event.preventDefault();
    dragImportDepth += 1;
    isDragImportActive = true;
  }

  function onWindowDragOver(event: DragEvent): void {
    if (!hasFileTransfer(event)) {
      return;
    }

    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
    isDragImportActive = true;
  }

  function onWindowDragLeave(event: DragEvent): void {
    if (!isDragImportActive) {
      return;
    }

    event.preventDefault();
    dragImportDepth = Math.max(0, dragImportDepth - 1);
    if (dragImportDepth === 0) {
      isDragImportActive = false;
    }
  }

  function onWindowDrop(event: DragEvent): void {
    if (!hasFileTransfer(event)) {
      return;
    }

    event.preventDefault();
    clearDragImportState();
    const file = event.dataTransfer?.files.item(0);
    if (!file) {
      return;
    }

    void importTomlFile(file);
  }

  async function loadInitialState(): Promise<void> {
    isBootstrapping = true;
    errorMessage = "";

    try {
      const payload = await loadBootstrap(lang);
      catalogItems = payload.catalog.items;
      defaultToml = payload.defaultAicToml;

      if (
        !hasRestoredDraftFromStorage &&
        draft.outposts.length === 0 &&
        draft.supply.length === 0 &&
        draft.consumption.length === 0
      ) {
        applyToml(defaultToml);
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      isBootstrapping = false;
    }
  }

  async function resetToDefault(): Promise<void> {
    try {
      if (defaultToml.length === 0) {
        await loadInitialState();
        return;
      }

      applyToml(defaultToml);
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    }
  }

  function requestResetToDefault(): void {
    const confirmed = window.confirm(
      t(
        "重置会覆盖当前所有配置并恢复默认值，是否继续？",
        "Reset will overwrite current configuration and restore defaults. Continue?",
      ),
    );

    if (!confirmed) {
      return;
    }

    void resetToDefault();
  }

  async function importFromFile(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) {
      return;
    }

    try {
      await importTomlFile(file);
    } finally {
      input.value = "";
    }
  }

  function exportToml(): void {
    try {
      const toml = buildAicToml(draft);
      const blob = new Blob([toml], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "aic.toml";
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    }
  }

  const editorActions: EditorActions = {
    resetToDefault: requestResetToDefault,
    importFromFile,
    exportToml,
    setRegion: (region) => {
      draft = setRegion(draft, region);
    },
    setExternalPower: (value) => {
      draft = setExternalPower(draft, value);
    },
    supply: {
      add: () => {
        draft = addSupplyRow(draft, catalogItems[0]?.key ?? "");
      },
      remove: (index) => {
        draft = removeSupplyRow(draft, index);
      },
      setKey: (index, key) => {
        draft = setSupplyKey(draft, index, key);
      },
      setValue: (index, value) => {
        draft = setSupplyValue(draft, index, value);
      },
    },
    consumption: {
      add: () => {
        draft = addConsumptionRow(draft, catalogItems[0]?.key ?? "");
      },
      remove: (index) => {
        draft = removeConsumptionRow(draft, index);
      },
      setKey: (index, key) => {
        draft = setConsumptionKey(draft, index, key);
      },
      setValue: (index, value) => {
        draft = setConsumptionValue(draft, index, value);
      },
    },
    outposts: {
      add: () => {
        const next = addOutpost(draft, selectedOutpostIndex);
        draft = next.draft;
        selectedOutpostIndex = next.selectedOutpostIndex;
      },
      remove: (index) => {
        const next = removeOutpost(draft, selectedOutpostIndex, index);
        draft = next.draft;
        selectedOutpostIndex = next.selectedOutpostIndex;
      },
      select: (index) => {
        selectedOutpostIndex = index;
      },
      setField: (index, field, value) => {
        draft = setOutpostField(draft, index, field, value);
      },
    },
    prices: {
      add: (outpostIndex) => {
        draft = addPriceRow(draft, outpostIndex, catalogItems[0]?.key ?? "");
      },
      remove: (outpostIndex, priceIndex) => {
        draft = removePriceRow(draft, outpostIndex, priceIndex);
      },
      setKey: (outpostIndex, priceIndex, key) => {
        draft = setPriceKey(draft, outpostIndex, priceIndex, key);
      },
      setValue: (outpostIndex, priceIndex, value) => {
        draft = setPriceValue(draft, outpostIndex, priceIndex, value);
      },
    },
  };

  onMount(() => {
    void warmupWasmWorker().catch(() => undefined);

    const restored = restoreLocalState(STORAGE_CONFIG);
    if (restored.leftPaneRatio !== null) {
      leftPaneRatio = restored.leftPaneRatio;
    }

    if (restored.draft) {
      draft = restored.draft;
      selectedOutpostIndex = restored.draft.outposts.length > 0 ? 0 : -1;
      hasRestoredDraftFromStorage = true;
    }

    hasHydratedLocalState = true;

    solverController = createSolverController({
      debounceMs: AUTO_SOLVE_DEBOUNCE_MS,
      getSnapshot: () => ({ draft, lang, isBootstrapping }),
      toToml: buildAicToml,
      solve: async (solveLang, toml) => {
        const startedAt = performance.now();
        try {
          return await solveScenario(solveLang, toml);
        } finally {
          solveElapsedMs = Math.max(0, Math.round(performance.now() - startedAt));
        }
      },
      onSolvingChange: (next) => {
        isSolving = next;
        if (next) {
          solveElapsedMs = null;
        }
      },
      onErrorMessage: (next) => {
        errorMessage = next;
      },
      onSolved: (payload) => {
        result = payload;
      },
    });

    const mediaQuery = window.matchMedia(NARROW_LAYOUT_QUERY);
    const updateScreenMode = (): void => {
      isNarrowScreen = mediaQuery.matches;
      if (!isNarrowScreen) {
        activeTab = "editor";
      }
      if (isNarrowScreen) {
        stopSplitResize();
      }
    };

    updateScreenMode();
    mediaQuery.addEventListener("change", updateScreenMode);
    window.addEventListener("dragenter", onWindowDragEnter);
    window.addEventListener("dragover", onWindowDragOver);
    window.addEventListener("dragleave", onWindowDragLeave);
    window.addEventListener("drop", onWindowDrop);
    void loadInitialState();

    return () => {
      mediaQuery.removeEventListener("change", updateScreenMode);
      window.removeEventListener("dragenter", onWindowDragEnter);
      window.removeEventListener("dragover", onWindowDragOver);
      window.removeEventListener("dragleave", onWindowDragLeave);
      window.removeEventListener("drop", onWindowDrop);
      clearDragImportState();
      stopSplitResize();
      solverController?.dispose();
      solverController = null;
    };
  });

  $effect(() => {
    if (!solverController) {
      return;
    }

    draft;
    lang;
    isBootstrapping;
    solverController.scheduleAutoSolve();
  });

  $effect(() => {
    if (!hasHydratedLocalState) {
      return;
    }

    persistDraft(STORAGE_CONFIG.draftStorageKey, draft);
  });

  $effect(() => {
    if (!hasHydratedLocalState) {
      return;
    }

    persistLeftPaneRatio(STORAGE_CONFIG.paneRatioStorageKey, leftPaneRatio);
  });

  $effect(() => {
    const normalized = normalizeSelectedOutpostIndex(
      draft,
      selectedOutpostIndex,
    );
    if (normalized !== selectedOutpostIndex) {
      selectedOutpostIndex = normalized;
    }
  });
</script>

<div class="shell">
  {#if isNarrowScreen}
    <nav class="mobile-tabs" aria-label={t("页面分区", "Panel tabs")}>
      <button
        type="button"
        class:active={activeTab === "editor"}
        onclick={() => (activeTab = "editor")}
      >
        {t("输入", "Inputs")}
      </button>
      <button
        type="button"
        class:active={activeTab === "result"}
        onclick={() => (activeTab = "result")}
      >
        {t("评估", "Summary")}
      </button>
      <button
        type="button"
        class:active={activeTab === "graph"}
        onclick={() => (activeTab = "graph")}
      >
        {t("物流", "Flow")}
      </button>
    </nav>
  {/if}

  <main
    class={`workspace ${isNarrowScreen ? "mobile" : ""}`}
    bind:this={layoutElement}
    style={isNarrowScreen
      ? undefined
      : `--left-pane-width: ${(leftPaneRatio * 100).toFixed(2)}%`}
  >
    <section
      class={`panel editor ${isNarrowScreen && activeTab !== "editor" ? "tab-hidden" : ""}`}
    >
      <EditorPanel
        {lang}
        {draft}
        {catalogItems}
        {selectedOutpostIndex}
        isResetDisabled={isBootstrapping}
        actions={editorActions}
      />
    </section>

    {#if !isNarrowScreen}
      <div
        class={`splitter ${isDraggingSplitter ? "dragging" : ""}`}
        role="separator"
        aria-label={t("左右栏宽度调节", "Resize left and right columns")}
        aria-orientation="vertical"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(leftPaneRatio * 100)}
        onpointerdown={startSplitResize}
      ></div>

      <div class="right-pane">
        <section class="panel result">
          <ResultPanel
            {lang}
            {isBootstrapping}
            {isSolving}
            {solveElapsedMs}
            {result}
            {errorMessage}
          />
        </section>

        <section class="panel graph">
          <GraphPanel {lang} {result} />
        </section>
      </div>
    {:else}
      <section
        class={`panel result ${activeTab !== "result" ? "tab-hidden" : ""}`}
      >
        <ResultPanel
          {lang}
          {isBootstrapping}
          {isSolving}
          {solveElapsedMs}
          {result}
          {errorMessage}
        />
      </section>

      <section
        class={`panel graph ${activeTab !== "graph" ? "tab-hidden" : ""}`}
      >
        <GraphPanel {lang} {result} />
      </section>
    {/if}

    {#if isDragImportActive}
      <div class="drag-import-overlay" aria-live="polite">
        <div class="drag-import-card">
          <p class="drag-import-title">
            {t("松开即可导入 aic.toml", "Drop to import aic.toml")}
          </p>
          <p class="drag-import-subtitle">
            {t("支持拖入 .toml 文件。", "Drop any .toml file here.")}
          </p>
        </div>
      </div>
    {/if}
  </main>
</div>
