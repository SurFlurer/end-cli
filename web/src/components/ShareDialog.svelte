<script lang="ts">
  import { tick } from "svelte";
  import { toBlob } from "html-to-image";
  import IconActionButton from "./IconActionButton.svelte";
  import {
    copyPngBlobToClipboard,
    copyTextToClipboard,
  } from "../lib/clipboard";
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

  let exportFrame = $state<HTMLIFrameElement | null>(null);
  let exportWait: {
    token: string;
    resolve: () => void;
    reject: (reason?: unknown) => void;
    timeoutId: number;
  } | null = $state(null);

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

  function clearExportWait(): void {
    if (!exportWait) {
      return;
    }
    if (typeof window !== "undefined") {
      window.clearTimeout(exportWait.timeoutId);
    }
    exportWait = null;
  }

  function waitForExportReady(
    token: string,
    timeoutMs = 20_000,
  ): Promise<void> {
    if (typeof window === "undefined") {
      return Promise.reject(new Error("export is unavailable"));
    }

    clearExportWait();
    return new Promise<void>((resolve, reject) => {
      const timeoutId = window.setTimeout(() => {
        exportWait = null;
        reject(new Error(t("导出超时。", "Export timed out.")));
      }, timeoutMs);

      exportWait = {
        token,
        resolve: () => {
          window.clearTimeout(timeoutId);
          exportWait = null;
          resolve();
        },
        reject: (reason) => {
          window.clearTimeout(timeoutId);
          exportWait = null;
          reject(reason);
        },
        timeoutId,
      };
    });
  }

  function onMessage(event: MessageEvent): void {
    if (!open) {
      return;
    }

    if (
      typeof window !== "undefined" &&
      event.origin !== window.location.origin
    ) {
      return;
    }

    const data = event.data as unknown;
    if (!data || typeof data !== "object") {
      return;
    }

    const kind = (data as { kind?: unknown }).kind;
    const token = (data as { token?: unknown }).token;
    if (typeof kind !== "string" || typeof token !== "string") {
      return;
    }

    if (!exportWait || exportWait.token !== token) {
      return;
    }

    if (kind === "end2.export.ready") {
      exportWait.resolve();
      return;
    }

    if (kind === "end2.export.err") {
      const message = (data as { message?: unknown }).message;
      exportWait.reject(
        new Error(
          typeof message === "string"
            ? message
            : t("导出失败。", "Export failed."),
        ),
      );
    }
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
      if (!exportFrame) {
        previewError = t("导出组件未就绪。", "Export frame is not ready.");
        return;
      }

      const encoded = await encodeTomlToShareParam(tomlText);
      const token = `${Date.now()}-${Math.random().toString(16).slice(2)}`;

      const url = new URL(window.location.href);
      url.searchParams.set("s", encoded);
      url.searchParams.set("lang", lang);
      url.searchParams.set("token", token);
      url.searchParams.set("w", "1600");
      url.searchParams.set("h", "900");
      url.hash = "#export";

      exportFrame.src = url.toString();
      await waitForExportReady(token);

      const exportDocument = exportFrame.contentDocument;
      const element = exportDocument?.getElementById("logistics-flow-map");
      if (!element) {
        previewError = t("未找到导出物流图。", "Export flow map not found.");
        return;
      }

      const blob = await toBlob(element, { cacheBust: true });
      if (!blob) {
        previewError = t("截图失败。", "Failed to render screenshot.");
        return;
      }

      const pngBlob =
        blob.type === "image/png"
          ? blob
          : blob.slice(0, blob.size, "image/png");
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
      clearExportWait();
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

<svelte:window onkeydown={onKeyDown} onmessage={onMessage} />

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

      <iframe
        class="export-frame"
        title={t("导出", "Export")}
        bind:this={exportFrame}
        src="about:blank"
      ></iframe>

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

  .export-frame {
    position: fixed;
    left: -10000px;
    top: -10000px;
    width: 1600px;
    height: 900px;
    border: 0;
    opacity: 0;
    pointer-events: none;
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
