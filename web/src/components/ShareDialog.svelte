<script lang="ts">
  import { tick } from "svelte";
  import IconActionButton from "./IconActionButton.svelte";
  import {
    copyPngBlobToClipboard,
    copyTextToClipboard,
  } from "../lib/clipboard";
  import { exportCurrentFlowToPngBlob } from "../lib/export-flow";
  import { encodeTomlToShareParam } from "../lib/share-link";
  import type { LangTag } from "../lib/types";

  interface Props {
    open: boolean;
    lang: LangTag;
    tomlText: string;
    outputJsonText: string;
    onClose: () => void;
  }

  let { open, lang, tomlText, outputJsonText, onClose }: Props = $props();

  let previewBlob = $state<Blob | null>(null);
  let previewUrl = $state<string | null>(null);
  let previewError = $state<string>("");
  let isRendering = $state(false);

  let actionError = $state<string>("");
  let actionBusy = $state<"" | "link" | "image" | "bundle">("");
  let lastCopied = $state<"" | "link" | "image" | "bundle">("");

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  function clearPreview(): void {
    if (previewUrl && typeof URL !== "undefined") {
      URL.revokeObjectURL(previewUrl);
    }
    previewBlob = null;
    previewUrl = null;
    previewError = "";
  }

  async function renderPreview(): Promise<void> {
    if (!open) {
      return;
    }
    if (typeof document === "undefined") {
      previewError = t(
        "当前环境不支持截图预览。",
        "Preview is unavailable in this environment.",
      );
      return;
    }

    if (typeof window === "undefined") {
      previewError = t(
        "当前环境不支持截图预览。",
        "Preview is unavailable in this environment.",
      );
      return;
    }

    clearPreview();
    isRendering = true;
    previewError = "";

    try {
      const pngBlob = await exportCurrentFlowToPngBlob({
        width: 1600,
        height: 900,
      });

      previewBlob = pngBlob;
      previewUrl = URL.createObjectURL(pngBlob);
    } catch (error) {
      previewError = error instanceof Error ? error.message : String(error);
    } finally {
      isRendering = false;
    }
  }

  $effect(() => {
    if (!open) {
      clearPreview();
      actionError = "";
      actionBusy = "";
      lastCopied = "";
      return;
    }

    // render preview reads/writes some state, we don't want then tracked by this effect
    void (async () => {
      await tick();
      await renderPreview();
    })();
  });

  function onKeyDown(event: KeyboardEvent): void {
    if (!open) {
      return;
    }
    if (event.key !== "Escape") {
      return;
    }
    event.preventDefault();
    onClose();
  }

  function onBackdropPointerDown(event: PointerEvent): void {
    if (event.target !== event.currentTarget) {
      return;
    }
    onClose();
  }

  async function copyLink(): Promise<void> {
    const encoded = await encodeTomlToShareParam(tomlText);
    const url = new URL(window.location.href);
    url.searchParams.set("s", encoded);
    await copyTextToClipboard(url.toString());
    lastCopied = "link";
  }

  async function copyImage(): Promise<void> {
    if (!previewBlob) {
      await renderPreview();
    }
    if (!previewBlob) {
      throw new Error(t("暂无可复制图片。", "No image to copy."));
    }

    await copyPngBlobToClipboard(previewBlob);
    lastCopied = "image";
  }

  async function copyBundle(): Promise<void> {
    const json = outputJsonText.trim();
    if (json.length === 0) {
      throw new Error(t("暂无可复制输出 JSON。", "No output JSON to copy."));
    }

    const bundle = `# aic.toml\n\n${tomlText.trim()}\n\n# output.json\n\n${json}\n`;
    await copyTextToClipboard(bundle);
    lastCopied = "bundle";
  }

  async function runAction(
    kind: "link" | "image" | "bundle",
    fn: () => Promise<void>,
  ): Promise<void> {
    actionError = "";
    actionBusy = kind;
    try {
      await fn();
    } catch (error) {
      actionError = error instanceof Error ? error.message : String(error);
    } finally {
      actionBusy = "";
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

{#if open}
  <div
    class="dialog-backdrop"
    role="presentation"
    onpointerdown={onBackdropPointerDown}
  >
    <div
      class="dialog-card"
      role="dialog"
      aria-modal="true"
      aria-label={t("分享", "Share")}
    >
      <div class="dialog-head">
        <h2 class="dialog-title">{t("分享", "Share")}</h2>
        <IconActionButton
          icon="close"
          ariaLabel={t("关闭", "Close")}
          title={t("关闭", "Close")}
          onClick={onClose}
        />
      </div>

      <div class="preview">
        {#if isRendering}
          <p class="hint">{t("正在生成预览...", "Rendering preview...")}</p>
        {:else if previewUrl}
          <img
            class="preview-img"
            src={previewUrl}
            alt={t("物流图预览", "Flow map preview")}
          />
        {:else}
          <p class="hint">{previewError || t("暂无预览。", "No preview.")}</p>
        {/if}
      </div>

      <div class="actions">
        <button
          type="button"
          class="btn"
          disabled={actionBusy !== ""}
          onclick={() => {
            void runAction("link", copyLink);
          }}
        >
          {lastCopied === "link"
            ? t("已复制链接", "Link copied")
            : t("复制输入链接", "Copy input link")}
        </button>

        <button
          type="button"
          class="btn"
          disabled={actionBusy !== ""}
          onclick={() => {
            void runAction("image", copyImage);
          }}
        >
          {lastCopied === "image"
            ? t("已复制图片", "Image copied")
            : t("复制图片", "Copy image")}
        </button>

        <button
          type="button"
          class="btn"
          disabled={actionBusy !== "" || outputJsonText.trim().length === 0}
          onclick={() => {
            void runAction("bundle", copyBundle);
          }}
        >
          {lastCopied === "bundle"
            ? t("已复制 TOML+JSON", "TOML+JSON copied")
            : t("复制 TOML+JSON", "Copy TOML+JSON")}
        </button>
      </div>

      {#if actionError.trim().length > 0}
        <p class="error">{actionError}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    background: var(--overlay-backdrop);
    backdrop-filter: blur(2px);
    padding: var(--space-4);
  }

  .dialog-card {
    width: min(720px, calc(100vw - 2 * var(--space-4)));
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--panel-strong);
    box-shadow: var(--shadow-popover);
    padding: clamp(16px, 2.2vw, 22px);
    display: grid;
    gap: var(--space-3);
  }

  .dialog-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .dialog-title {
    font-size: 16px;
    letter-spacing: -0.01em;
  }

  .preview {
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--panel);
    padding: var(--space-2);
    min-height: 180px;
    display: grid;
    place-items: center;
  }

  .preview-img {
    max-width: 100%;
    max-height: min(52vh, 520px);
    border-radius: var(--radius-sm);
    display: block;
  }


  .hint {
    margin: 0;
    color: var(--muted-text);
    line-height: 1.4;
    font-size: 14px;
    text-align: center;
  }

  .actions {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }

  @media (max-width: 520px) {
    .actions {
      grid-template-columns: 1fr;
    }
  }

  .btn {
    border: 1px solid var(--control-border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    background: var(--panel-strong);
    color: inherit;
    font: inherit;
    line-height: 1.2;
    cursor: pointer;
    text-align: center;
    white-space: nowrap;
  }

  @media (hover: hover) and (pointer: fine) {
    .btn:hover:not(:disabled) {
      background: var(--surface-soft);
    }
  }

  .btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }

  .error {
    margin: 0;
    color: var(--danger);
    line-height: 1.4;
    font-size: 14px;
  }
</style>
