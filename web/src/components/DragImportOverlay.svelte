<script lang="ts">
  import { onMount } from "svelte";
  import type { LangTag } from "../lib/types";

  type DragImportOverlayProps = {
    lang: LangTag;
    onImportFile: (file: File) => void | Promise<void>;
  };

  let { lang, onImportFile }: DragImportOverlayProps = $props();

  let isDragImportActive = $state(false);
  let dragImportDepth = 0;

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
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

    void onImportFile(file);
  }

  onMount(() => {
    window.addEventListener("dragenter", onWindowDragEnter);
    window.addEventListener("dragover", onWindowDragOver);
    window.addEventListener("dragleave", onWindowDragLeave);
    window.addEventListener("drop", onWindowDrop);

    return () => {
      window.removeEventListener("dragenter", onWindowDragEnter);
      window.removeEventListener("dragover", onWindowDragOver);
      window.removeEventListener("dragleave", onWindowDragLeave);
      window.removeEventListener("drop", onWindowDrop);
      clearDragImportState();
    };
  });
</script>

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

<style>
  .drag-import-overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, #0b1612 26%, transparent);
    backdrop-filter: blur(2px);
    pointer-events: none;
  }

  .drag-import-card {
    width: min(560px, calc(100vw - 36px));
    border: 2px dashed color-mix(in srgb, var(--accent) 64%, #b7ddd0);
    border-radius: 20px;
    background: color-mix(in srgb, var(--panel) 86%, #f2fff9);
    box-shadow: 0 24px 56px rgba(7, 35, 24, 0.24);
    padding: clamp(22px, 3vw, 34px);
    text-align: center;
    display: grid;
    gap: var(--space-2);
  }

  .drag-import-title {
    margin: 0;
    font-size: clamp(1.05rem, 2.1vw, 1.45rem);
    font-weight: 700;
    color: color-mix(in srgb, #0c2017 85%, var(--accent));
  }

  .drag-import-subtitle {
    margin: 0;
    color: var(--muted-text);
    font-size: clamp(0.86rem, 1.6vw, 1rem);
  }
</style>