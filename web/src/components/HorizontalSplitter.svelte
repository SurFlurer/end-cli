<script lang="ts">
  import { onDestroy } from "svelte";

  interface Props {
    layoutElement: HTMLElement | null;
    ratio: number;
    minTopPx: number;
    minBottomPx: number;
    ariaLabel: string;
    onRatioChange: (nextRatio: number) => void;
  }

  let {
    layoutElement,
    ratio,
    minTopPx,
    minBottomPx,
    ariaLabel,
    onRatioChange,
  }: Props = $props();

  let splitterElement = $state<HTMLDivElement | null>(null);
  let isDragging = $state(false);
  let dragAnchorPx = $state(0);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function toAnchor(
    clientY: number,
    layoutTop: number,
    usableHeight: number,
  ): number {
    if (usableHeight <= 0) {
      return 0;
    }
    return clientY - layoutTop - ratio * usableHeight;
  }

  function splitterHeight(): number {
    return splitterElement?.getBoundingClientRect().height ?? 0;
  }

  function updateRatioByClientY(clientY: number): void {
    if (!layoutElement) {
      return;
    }

    const rect = layoutElement.getBoundingClientRect();
    const usableHeight = rect.height - splitterHeight();
    if (usableHeight <= 0) {
      return;
    }

    if (usableHeight <= minTopPx + minBottomPx) {
      onRatioChange(0.5);
      return;
    }

    const minRatio = minTopPx / usableHeight;
    const maxRatio = 1 - minBottomPx / usableHeight;
    const nextRatio = (clientY - rect.top - dragAnchorPx) / usableHeight;
    onRatioChange(clamp(nextRatio, minRatio, maxRatio));
  }

  function onPointerMove(event: PointerEvent): void {
    if (!isDragging) {
      return;
    }
    updateRatioByClientY(event.clientY);
  }

  function stopResize(): void {
    if (!isDragging) {
      return;
    }

    isDragging = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  function startResize(event: PointerEvent): void {
    event.preventDefault();
    isDragging = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";

    if (layoutElement) {
      const rect = layoutElement.getBoundingClientRect();
      const usableHeight = rect.height - splitterHeight();
      dragAnchorPx = toAnchor(event.clientY, rect.top, usableHeight);
    } else {
      dragAnchorPx = 0;
    }

    updateRatioByClientY(event.clientY);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
  }

  onDestroy(() => {
    stopResize();
  });
</script>

<div
  bind:this={splitterElement}
  class={`splitter ${isDragging ? "dragging" : ""}`}
  role="separator"
  aria-label={ariaLabel}
  aria-orientation="horizontal"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(ratio * 100)}
  onpointerdown={startResize}
></div>

<style>
  .splitter {
    grid-area: splitter-h;
    position: relative;
    background: transparent;
    border: none;
    cursor: row-resize;
    touch-action: none;
  }

  .splitter::after {
    content: "";
    position: absolute;
    inset-inline: 0;
    top: 50%;
    height: 2px;
    transform: translateY(-50%) scaleX(0.6);
    border-radius: 999px;
    background: var(--accent);
    opacity: 0;
    transition:
      opacity 60ms ease,
      transform 120ms ease;
    pointer-events: none;
  }

  .splitter:hover::after,
  .splitter.dragging::after {
    opacity: 1;
    transform: translateY(-50%) scaleX(1);
  }
</style>
