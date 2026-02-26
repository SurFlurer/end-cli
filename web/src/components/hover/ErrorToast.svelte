<script lang="ts">
  import IconActionButton from "../button/IconActionButton.svelte";

  export type ErrorToastState =
    | { kind: "closed" }
    | { kind: "open"; message: string };

  interface Props {
    state: ErrorToastState;
    onClose: () => void;
    title?: string;
  }

  let { state, onClose, title = "Error" }: Props = $props();

  const message = $derived(state.kind === "open" ? state.message : "");
  const trimmedMessage = $derived(message.trim());
  const isOpen = $derived(state.kind === "open" && trimmedMessage.length > 0);
</script>

{#if isOpen}
  <div class="toast" role="status" aria-live="polite" aria-label={title}>
    <div class="icon" aria-hidden="true">
      <span class="material-symbols-outlined">error</span>
    </div>

    <div class="content">
      <p class="title">{title}</p>
      <p class="message">{trimmedMessage}</p>
    </div>

    <div class="actions">
      <IconActionButton
        ariaLabel="Close"
        icon="close"
        onClick={onClose}
      />
    </div>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    top: calc(var(--space-4) + env(safe-area-inset-top));
    left: 50%;
    transform: translateX(-50%);
    z-index: 900;
    width: min(520px, calc(100vw - 2 * var(--space-3)));
    border-radius: var(--radius-lg);
    background: var(--panel-strong);
    box-shadow: var(--shadow-popover);
    padding: var(--space-3);
    display: grid;
    grid-template-columns: 20px 1fr auto;
    align-items: start;
    gap: var(--space-2);

    animation: toast-in 160ms ease;
    will-change: transform, opacity;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(10px) scale(0.98);
    }

    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
      will-change: auto;
    }
  }

  .icon {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    color: var(--danger);
    margin-top: 2px;
  }

  .icon span {
    font-size: 20px;
    font-variation-settings:
      "FILL" 0,
      "wght" 600,
      "GRAD" 0,
      "opsz" 20;
  }

  .content {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .title {
    font-size: 12px;
    letter-spacing: 0.01em;
    font-weight: 700;
    color: var(--danger);
  }

  .message {
    font-size: 13px;
    line-height: 1.35;
    color: var(--ink);
    overflow-wrap: anywhere;
  }

  .actions {
    margin-top: -6px;
  }
</style>
