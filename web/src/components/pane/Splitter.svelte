<script lang="ts">
  import { onDestroy } from "svelte";

  interface Props {
    layoutElement: HTMLElement | null;
    ratio: number;
    minLeftPx: number;
    minRightPx: number;
    ariaLabel: string;
    onRatioChange: (nextRatio: number) => void;
  }

  let {
    layoutElement,
    ratio,
    minLeftPx,
    minRightPx,
    ariaLabel,
    onRatioChange,
  }: Props = $props();

  let isDragging = $state(false);
  let dragAnchorPx = $state(0);
  let splitterElement = $state<HTMLDivElement | null>(null);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function toAnchor(
    clientX: number,
    layoutLeft: number,
    usableWidth: number,
  ): number {
    if (usableWidth <= 0) {
      return 0;
    }
    return clientX - layoutLeft - ratio * usableWidth;
  }

  function splitterWidth(): number {
    return splitterElement?.getBoundingClientRect().width ?? 0;
  }

  function updateRatioByClientX(clientX: number): void {
    if (!layoutElement) {
      return;
    }

    const rect = layoutElement.getBoundingClientRect();
    const usableWidth = rect.width - splitterWidth();
    if (usableWidth <= 0) {
      return;
    }

    if (usableWidth <= minLeftPx + minRightPx) {
      onRatioChange(0.5);
      return;
    }

    const minRatio = minLeftPx / usableWidth;
    const maxRatio = 1 - minRightPx / usableWidth;
    const nextRatio = (clientX - rect.left - dragAnchorPx) / usableWidth;
    onRatioChange(clamp(nextRatio, minRatio, maxRatio));
  }

  function onPointerMove(event: PointerEvent): void {
    if (!isDragging) {
      return;
    }
    updateRatioByClientX(event.clientX);
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
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    if (layoutElement) {
      const rect = layoutElement.getBoundingClientRect();
      const usableWidth = rect.width - splitterWidth();
      dragAnchorPx = toAnchor(event.clientX, rect.left, usableWidth);
    } else {
      dragAnchorPx = 0;
    }

    updateRatioByClientX(event.clientX);
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
  aria-orientation="vertical"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(ratio * 100)}
  onpointerdown={startResize}
></div>

<style>
  .splitter {
    grid-area: splitter;
    position: relative;
    background: transparent;
    border: none;
    cursor: col-resize;
    touch-action: none;
  }

  .splitter::after {
    content: "";
    position: absolute;
    inset-block: 0;
    left: 50%;
    width: 2px;
    transform: translateX(-50%) scaleY(0.6);
    border-radius: 999px;
    background: var(--accent);
    opacity: 0;
    transition:
      /* 更快出现，更慢垂直展开 */
      opacity 60ms ease,
      transform 120ms ease;
    pointer-events: none;
  }

  .splitter:hover,
  .splitter.dragging {
    background: transparent;
  }

  .splitter:hover::after,
  .splitter.dragging::after {
    opacity: 1;
    transform: translateX(-50%) scaleY(1);
  }
</style>
