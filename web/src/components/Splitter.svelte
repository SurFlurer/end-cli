<script lang="ts">
  import { onDestroy } from "svelte";

  interface Props {
    layoutElement: HTMLElement | null;
    ratio: number;
    minLeftPx: number;
    minRightPx: number;
    splitterWidthPx: number;
    ariaLabel: string;
    onRatioChange: (nextRatio: number) => void;
  }

  let {
    layoutElement,
    ratio,
    minLeftPx,
    minRightPx,
    splitterWidthPx,
    ariaLabel,
    onRatioChange,
  }: Props = $props();

  let isDragging = $state(false);
  let dragAnchorPx = $state(0);

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

  function updateRatioByClientX(clientX: number): void {
    if (!layoutElement) {
      return;
    }

    const rect = layoutElement.getBoundingClientRect();
    const usableWidth = rect.width - splitterWidthPx;
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
      const usableWidth = rect.width - splitterWidthPx;
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
    border-radius: 999px;
    background: linear-gradient(180deg, #d8e9de 0%, #b7d2c4 100%);
    border: 1px solid #a6c6b6;
    cursor: col-resize;
    touch-action: none;
  }

  .splitter:hover,
  .splitter.dragging {
    background: linear-gradient(180deg, #c1e2d2 0%, #8dbca7 100%);
  }
</style>
