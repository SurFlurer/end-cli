<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    contentMode?: "default" | "flush";
    header?: Snippet;
    children?: Snippet;
  }

  let { contentMode = "default", header, children }: Props = $props();
</script>

<div class="panel-frame">
  <header class="panel-header">
    {@render header?.()}
  </header>
  <div class="panel-content" class:flush={contentMode === "flush"}>
    {@render children?.()}
  </div>
</div>

<style>
  .panel-frame {
    border-radius: var(--radius-md);
    background: var(--panel);
    box-shadow: var(--shadow-panel);
    min-height: 0;
    height: 100%;
    overflow: hidden;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
  }

  .panel-header {
    padding: clamp(12px, 1.6vw, 16px);
    border-bottom: 1px solid color-mix(in srgb, var(--line) 70%, var(--line-tint-1));
    min-width: 0;
  }

  .panel-content {
    min-height: 0;
    min-width: 0;
    padding: clamp(12px, 1.6vw, 16px);
    display: grid;
    gap: var(--space-3);
    align-content: start;
    overflow-y: hidden;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--line) 78%, var(--accent)) color-mix(in srgb, var(--surface-soft) 72%, transparent);
  }

  .panel-frame:hover .panel-content,
  .panel-frame:focus-within .panel-content {
    overflow-y: auto;
    scrollbar-color: color-mix(in srgb, var(--accent) 44%, var(--panel-strong)) color-mix(in srgb, var(--surface-soft) 72%, transparent);
  }

  .panel-content::-webkit-scrollbar {
    width: 10px;
    height: 10px;
  }

  .panel-content::-webkit-scrollbar-track {
    background: color-mix(in srgb, var(--surface-soft) 72%, transparent);
  }

  .panel-content::-webkit-scrollbar-thumb {
    border-radius: var(--radius-sm);
    border: 2px solid color-mix(in srgb, var(--surface-soft) 72%, transparent);
    background: color-mix(in srgb, var(--line) 78%, var(--accent));
  }

  .panel-frame:hover .panel-content::-webkit-scrollbar-thumb,
  .panel-frame:focus-within .panel-content::-webkit-scrollbar-thumb {
    background: color-mix(in srgb, var(--accent) 44%, var(--panel-strong));
  }

  /* Touch devices don't have hover, so keep content scrollable. */
  @media (hover: none) {
    .panel-content {
      overflow-y: auto;
      touch-action: pan-y;
      -webkit-overflow-scrolling: touch;
    }
  }

  .panel-content.flush {
    padding: 0;
    gap: 0;
    align-content: stretch;
    overflow: hidden;
  }
</style>