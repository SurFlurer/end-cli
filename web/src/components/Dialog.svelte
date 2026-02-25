<script lang="ts">
  import { tick } from "svelte";

  interface Props {
    open: boolean;
    title: string;
    description?: string;
    kind?: "default" | "danger";
    cancelLabel?: string;
    confirmLabel?: string;
    confirmDisabled?: boolean;
    onCancel: () => void;
    onConfirm: () => void | Promise<void>;
  }

  let {
    open,
    title,
    description = "",
    kind = "default",
    cancelLabel = "Cancel",
    confirmLabel = "OK",
    confirmDisabled = false,
    onCancel,
    onConfirm,
  }: Props = $props();

  let cancelButton = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    if (!open) {
      return;
    }

    void (async () => {
      await tick();
      cancelButton?.focus();
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
    onCancel();
  }

  function onBackdropPointerDown(event: PointerEvent): void {
    if (event.target !== event.currentTarget) {
      return;
    }
    onCancel();
  }

  async function handleConfirm(): Promise<void> {
    if (confirmDisabled) {
      return;
    }
    await onConfirm();
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
      class={`dialog-card ${kind === "danger" ? "danger" : ""}`}
      role="dialog"
      aria-modal="true"
      aria-label={title}
    >
      <h2 class="dialog-title">{title}</h2>
      {#if description.trim().length > 0}
        <p class="dialog-desc">{description}</p>
      {/if}
      <div class="dialog-actions">
        <button type="button" class="btn" onclick={onCancel} bind:this={cancelButton}>
          {cancelLabel}
        </button>
        <button
          type="button"
          class={`btn primary ${kind === "danger" ? "danger" : ""}`}
          onclick={handleConfirm}
          disabled={confirmDisabled}
        >
          {confirmLabel}
        </button>
      </div>
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
    width: min(520px, calc(100vw - 2 * var(--space-4)));
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--panel-strong);
    box-shadow: var(--shadow-popover);
    padding: clamp(16px, 2.2vw, 22px);
    display: grid;
    gap: var(--space-2);
  }

  .dialog-title {
    font-size: 16px;
    letter-spacing: -0.01em;
  }

  .dialog-desc {
    margin: 0;
    color: var(--muted-text);
    line-height: 1.4;
    font-size: 14px;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .btn {
    border: 1px solid var(--control-border);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    background: var(--panel-strong);
    color: inherit;
    font: inherit;
    line-height: 1.2;
    cursor: pointer;
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

  .btn.primary {
    border-color: transparent;
    background: var(--accent);
    color: var(--panel-strong);
  }

  @media (hover: hover) and (pointer: fine) {
    .btn.primary:hover:not(:disabled) {
      background: color-mix(in srgb, var(--accent) 82%, var(--ink));
    }
  }

  .btn.primary.danger {
    background: var(--danger);
  }

  @media (hover: hover) and (pointer: fine) {
    .btn.primary.danger:hover:not(:disabled) {
      background: color-mix(in srgb, var(--danger) 84%, var(--ink));
    }
  }
</style>
