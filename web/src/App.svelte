<script lang="ts">
  import Dialog from "./components/Dialog.svelte";
  import ErrorToast, { type ErrorToastState } from "./components/ErrorToast.svelte";
  import { onMount } from "svelte";
  import EditorPanel from "./components/EditorPanel.svelte";
  import DragImportOverlay from "./components/DragImportOverlay.svelte";
  import GraphPanel from "./components/GraphPanel.svelte";
  import HorizontalSplitter from "./components/HorizontalSplitter.svelte";
  import ResultPanel from "./components/ResultPanel.svelte";
  import Splitter from "./components/Splitter.svelte";
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
    setStage2Objective,
    setStage2Weight,
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
  import {
    type SolveState,
  } from "./lib/solve-state";
  import type {
    AicDraft,
    CatalogItemDto,
    LangTag,
  } from "./lib/types";
  import { EMPTY_DRAFT } from "./lib/types";
  import { loadBootstrap, solveScenario, warmupWasmWorker } from "./lib/wasm";

  const NARROW_LAYOUT_QUERY = "(max-width: 1160px)";
  const MIN_EDITOR_WIDTH_PX = 720;
  const MIN_RIGHT_WIDTH_PX = 420;
  const MIN_TOP_PANEL_HEIGHT_PX = 80;
  const MIN_BOTTOM_PANEL_HEIGHT_PX = 80+12;
  const AUTO_SOLVE_DEBOUNCE_MS = 200;

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

  let lang = $state<LangTag>(detectBrowserLang());
  let catalogItems = $state<CatalogItemDto[]>([]);
  let draft = $state<AicDraft>(structuredClone(EMPTY_DRAFT));
  let defaultToml = $state("");

  let isBootstrapping = $state(true);
  let solveState = $state<SolveState>({ status: "idle" });

  let errorToast = $state<ErrorToastState>({ kind: "closed" });

  let selectedOutpostIndex = $state(-1);

  let layoutElement = $state<HTMLElement | null>(null);
  let rightPaneElement = $state<HTMLElement | null>(null);
  let isNarrowScreen = $state(false);
  let activeTab = $state<"editor" | "result" | "graph">("editor");
  let leftPaneRatio = $state(0.55);
  let rightPaneRatio = $state(0.5);

  let hasHydratedLocalState = $state(false);
  let hasRestoredDraftFromStorage = $state(false);

  let solverController: SolverController | null = null;

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function showErrorToast(message: string): void {
    const trimmed = message.trim();
    if (trimmed.length === 0) {
      return;
    }
    errorToast = { kind: "open", message: trimmed };
  }

  function closeErrorToast(): void {
    errorToast = { kind: "closed" };
  }

  function applyToml(text: string): void {
    try {
      const nextDraft = parseAicToml(text);
      draft = nextDraft;
      selectedOutpostIndex = nextDraft.outposts.length > 0 ? 0 : -1;
      solverController?.resetSolvedFingerprint();
    } catch (error) {
      showErrorToast(error instanceof Error ? error.message : String(error));
    }
  }

  function isTomlFile(file: File): boolean {
    return file.name.trim().toLowerCase().endsWith(".toml");
  }

  async function importTomlFile(file: File): Promise<void> {
    if (!isTomlFile(file)) {
      showErrorToast(
        t(
          "仅支持导入 *.toml 格式文件。",
          "Only *.toml files are supported for import.",
        ),
      );
      return;
    }

    try {
      const text = await file.text();
      applyToml(text);
    } catch (error) {
      showErrorToast(error instanceof Error ? error.message : String(error));
    }
  }

  async function loadInitialState(): Promise<void> {
    isBootstrapping = true;

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
      showErrorToast(error instanceof Error ? error.message : String(error));
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
      showErrorToast(error instanceof Error ? error.message : String(error));
    }
  }

  function requestResetToDefault(): void {
    isResetDialogOpen = true;
  }

  let isResetDialogOpen = $state(false);

  function closeResetDialog(): void {
    isResetDialogOpen = false;
  }

  async function confirmResetToDefault(): Promise<void> {
    closeResetDialog();
    await resetToDefault();
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
      showErrorToast(error instanceof Error ? error.message : String(error));
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
    setStage2Objective: (objective) => {
      draft = setStage2Objective(draft, objective);
    },
    setStage2Weight: (field, value) => {
      draft = setStage2Weight(draft, field, value);
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
        const next = addOutpost(draft);
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
      toToml: buildAicToml,
      solve: (solveLang, toml) => solveScenario(solveLang, toml),
      onStateChange: (next) => {
        solveState = next;
      },
    });

    const mediaQuery = window.matchMedia(NARROW_LAYOUT_QUERY);
    const updateScreenMode = (): void => {
      isNarrowScreen = mediaQuery.matches;
      if (!isNarrowScreen) {
        activeTab = "editor";
      }
    };

    updateScreenMode();
    mediaQuery.addEventListener("change", updateScreenMode);
    void loadInitialState();

    return () => {
      mediaQuery.removeEventListener("change", updateScreenMode);
      solverController?.dispose();
      solverController = null;
    };
  });

  $effect(() => {
    if (!solverController) {
      return;
    }

    solverController.updateSnapshot({ draft, lang, isBootstrapping });
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
      class={`editor ${isNarrowScreen && activeTab !== "editor" ? "tab-hidden" : ""}`}
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
      <Splitter
        {layoutElement}
        ratio={leftPaneRatio}
        minLeftPx={MIN_EDITOR_WIDTH_PX}
        minRightPx={MIN_RIGHT_WIDTH_PX}
        ariaLabel={t("左右栏宽度调节", "Resize left and right columns")}
        onRatioChange={(nextRatio) => {
          leftPaneRatio = nextRatio;
        }}
      />

      <div
        class="right-pane"
        bind:this={rightPaneElement}
        style={`--right-top-height: ${(rightPaneRatio * 100).toFixed(2)}%; --right-min-top-height: ${MIN_TOP_PANEL_HEIGHT_PX}px; --right-min-bottom-height: ${MIN_BOTTOM_PANEL_HEIGHT_PX}px`}
      >
        <ResultPanel
          {lang}
          {isBootstrapping}
          {solveState}
        />

        <HorizontalSplitter
          layoutElement={rightPaneElement}
          ratio={rightPaneRatio}
          minTopPx={MIN_TOP_PANEL_HEIGHT_PX}
          minBottomPx={MIN_BOTTOM_PANEL_HEIGHT_PX}
          ariaLabel={t("上下栏高度调节", "Resize top and bottom panels")}
          onRatioChange={(nextRatio) => {
            rightPaneRatio = nextRatio;
          }}
        />

        <GraphPanel {lang} {solveState} />
      </div>
    {:else}
      <section
        class={`${activeTab !== "result" ? "tab-hidden" : ""}`}
      >
        <ResultPanel
          {lang}
          {isBootstrapping}
          {solveState}
        />
      </section>

      <section
        class={`${activeTab !== "graph" ? "tab-hidden" : ""}`}
      >
        <GraphPanel {lang} {solveState} />
      </section>
    {/if}

    <DragImportOverlay {lang} onImportFile={importTomlFile} />
  </main>
</div>

<ErrorToast
  state={errorToast}
  title={t("错误", "Error")}
  onClose={closeErrorToast}
/>

<Dialog
  open={isResetDialogOpen}
  kind="danger"
  title={t("重置为默认输入", "Reset to Default Input")}
  description={t(
    "重置会覆盖当前所有配置并恢复默认值，是否继续？",
    "Reset will overwrite current configuration and restore defaults. Continue?",
  )}
  cancelLabel={t("取消", "Cancel")}
  confirmLabel={t("重置", "Reset")}
  confirmDisabled={isBootstrapping}
  onCancel={closeResetDialog}
  onConfirm={confirmResetToDefault}
/>
