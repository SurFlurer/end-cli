<script lang="ts">
  import { tick } from "svelte";

  let { text }: { text: string } = $props();

  let wrapEl = $state<HTMLSpanElement | null>(null);
  let bubbleEl = $state<HTMLSpanElement | null>(null);
  let isOpen = $state(false);
  let bubbleStyle = $state("");
  let bubblePlacement = $state<"top" | "bottom">("top");

  async function openHint(): Promise<void> {
    isOpen = true;
    await tick();
    repositionHint();
  }

  function closeHint(): void {
    isOpen = false;
  }

  function onViewportChange(): void {
    if (!isOpen) {
      return;
    }
    repositionHint();
  }

  function repositionHint(): void {
    if (!wrapEl || !bubbleEl) {
      return;
    }

    const triggerRect = wrapEl.getBoundingClientRect();
    const bubbleRect = bubbleEl.getBoundingClientRect();
    const edgePadding = 12;
    const gap = 10;

    let left =
      triggerRect.left + triggerRect.width / 2 - bubbleRect.width / 2;
    left = Math.max(
      edgePadding,
      Math.min(left, window.innerWidth - bubbleRect.width - edgePadding),
    );

    let top = triggerRect.top - bubbleRect.height - gap;
    bubblePlacement = "top";
    if (top < edgePadding) {
      top = triggerRect.bottom + gap;
      bubblePlacement = "bottom";
    }

    bubbleStyle = `left: ${left}px; top: ${top}px;`;
  }
</script>

<svelte:window onresize={onViewportChange} onscroll={onViewportChange} />

<span class="hint-wrap" bind:this={wrapEl}>
  <button
    type="button"
    class="hint-trigger"
    aria-label={text}
    aria-expanded={isOpen}
    onmouseenter={openHint}
    onmouseleave={closeHint}
    onfocus={openHint}
    onblur={closeHint}
  >
    <span class="material-symbols-outlined icon" aria-hidden="true">info</span>
  </button>
  <span
    bind:this={bubbleEl}
    class={`hint-bubble ${isOpen ? "open" : ""} ${bubblePlacement}`}
    role="tooltip"
    style={bubbleStyle}
    aria-hidden={!isOpen}
  >
    {text}
  </span>
</span>

<style>
  .hint-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    isolation: isolate;
  }

  .hint-trigger {
    border: none;
    background: var(--panel-strong);
    color: var(--accent-ink);
    border-radius: 999px;
    width: 24px;
    height: 24px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    cursor: help;
  }

  @media (hover: hover) and (pointer: fine) {
    .hint-trigger:hover {
      background: color-mix(in srgb,var(--surface-soft) 60%,var(--accent-soft));
      color: var(--ink);
    }
  }

  .hint-trigger:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .hint-trigger .icon {
    font-size: 16px;
    line-height: 1;
    display: block;
    font-variation-settings:
      "FILL" 0,
      "wght" 500,
      "GRAD" 0,
      "opsz" 20;
  }

  .hint-bubble {
    position: fixed;
    min-width: 260px;
    /* compatible with narrower screen */
    max-width: min(380px, calc(100vw - 36px));
    border-radius: var(--radius-sm);
    background: var(--panel-strong);
    color: var(--ink-soft);
    box-shadow: var(--shadow-popover);
    padding: 10px 12px;
    font-size: 14px;
    line-height: 1.35;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
    z-index: 2000;
  }

  .hint-bubble.open {
    opacity: 1;
  }
</style>
