<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    titleText?: string;
    subtitleText?: string;

    title?: Snippet;
    subtitle?: Snippet;
    controls?: Snippet;
  }

  let {
    titleText,
    subtitleText,
    title,
    subtitle,
    controls,
  }: Props = $props();
</script>

<div class="panel-header-layout">
  <div class="left">
    {#if title}
      {@render title()}
    {:else if titleText}
      <h2>{titleText}</h2>
    {/if}

    {#if subtitle}
      <p class="subtitle">{@render subtitle()}</p>
    {:else if subtitleText}
      <p class="subtitle">{subtitleText}</p>
    {/if}
  </div>

  {#if controls}
    <div class="controls">{@render controls()}</div>
  {/if}
</div>

<style>
  .panel-header-layout {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
    min-width: 0;
    align-items: center;
  }

  .left {
    min-width: 0;
  }

  .subtitle {
    margin-top: var(--space-1);
    color: var(--muted-text);
    font-size: 12px;
    line-height: 1.35;
  }

  .controls {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
</style>
