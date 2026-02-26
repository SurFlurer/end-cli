<script lang="ts">
  import { tooltip } from "../../lib/tooltip";

  interface Props {
    isCollapsed: boolean;
    label: string;
    onToggle: () => void;
  }

  let { isCollapsed, label, onToggle }: Props = $props();
</script>

<button
  type="button"
  class="brand-handle"
  aria-label={label}
  use:tooltip={label}
  onclick={onToggle}
>
  <span
    class="material-symbols-outlined icon handle-icon"
    class:rotated={isCollapsed}
    aria-hidden="true"
  >expand_more</span>
  <img src="/favicon.svg" alt="" width="24" height="24" aria-hidden="true" />
</button>

<style>
  .brand-handle {
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: inherit;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: var(--control-size);
    line-height: 1;
  }

  .brand-handle:hover {
    background: color-mix(in srgb, var(--surface-soft) 60%, var(--accent-soft));
  }

  .handle-icon {
    transition: transform 160ms ease;
    transform: rotate(0deg);
  }

  .handle-icon.rotated {
    transform: rotate(180deg);
  }

  .icon {
    font-size: 20px;
    line-height: 1;
    display: block;
    font-variation-settings:
      "FILL" 0,
      "wght" 400,
      "GRAD" 0,
      "opsz" 20;
  }
</style>
