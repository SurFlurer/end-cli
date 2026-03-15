<script lang="ts">
  import type { Snippet } from "svelte";
  import FieldHint from "../hover/FieldHint.svelte";

  interface Props {
    titleText?: string;
    icon?: string;
    fieldHintText?: string;

    title?: Snippet;
    controls?: Snippet;
  }

  let {
    titleText,
    icon,
    fieldHintText,
    title,
    controls,
  }: Props = $props();
</script>

<div class="panel-header-layout">
  <div class="left">
    {#if title}
      {@render title()}
    {:else if titleText}
      <h2 class="panel-title">
        {#if icon}
          <span class="material-symbols-outlined panel-icon">{icon}</span>
        {/if}
        <span class="title-text">{titleText}</span>
        {#if fieldHintText}
          <FieldHint text={fieldHintText} />
        {/if}
      </h2>
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
    display: inline-flex;
    min-width: 0;
  }

  .panel-title {
    display: inline-flex;
    gap: var(--space-2);
    margin: 0;
    font-size: 15px;
    color: var(--ink);
    line-height: 1.4;
  }

  .panel-icon {
    font-size: 24px;
    color: var(--ink);
    font-variation-settings:
      "FILL" 0,
      "wght" 400,
      "GRAD" 0,
      "opsz" 48;
  }

  .title-text {
    font-weight: 500;
  }

  .controls {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
</style>
