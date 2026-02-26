<script lang="ts">
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import IconActionButton from "../button/IconActionButton.svelte";

  interface Props {
    menuAriaLabel: string;
    triggerAriaLabel: string;
    triggerTitle?: string;
    triggerIcon?: string;
    disabled?: boolean;
    menu: Snippet<[close: () => void]>;
  }

  let {
    menuAriaLabel,
    triggerAriaLabel,
    triggerTitle = "",
    triggerIcon = "more_vert",
    disabled = false,
    menu,
  }: Props = $props();

  let isOpen = $state(false);
  let root = $state<HTMLElement | null>(null);

  function close(): void {
    isOpen = false;
  }

  function toggle(): void {
    if (disabled) {
      return;
    }
    isOpen = !isOpen;
  }

  function onWindowPointerDown(event: PointerEvent): void {
    if (!isOpen) {
      return;
    }
    const target = event.target;
    if (!(target instanceof Node)) {
      return;
    }
    if (root && root.contains(target)) {
      return;
    }
    close();
  }

  function onWindowKeyDown(event: KeyboardEvent): void {
    if (!isOpen) {
      return;
    }
    if (event.key !== "Escape") {
      return;
    }
    event.preventDefault();
    close();
  }

  onMount(() => {
    return () => {
      close();
    };
  });
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydown={onWindowKeyDown} />

<div class="menu-root" bind:this={root}>
  <IconActionButton
    icon={triggerIcon}
    onClick={toggle}
    active={isOpen}
    title={triggerTitle}
    ariaLabel={triggerAriaLabel}
    disabled={disabled}
  />

  {#if isOpen}
    <div class="dropdown" role="menu" aria-label={menuAriaLabel}>
      {@render menu(close)}
    </div>
  {/if}
</div>

<style>
  .menu-root {
    position: relative;
    display: inline-flex;
  }

  .dropdown {
    position: absolute;
    top: calc(var(--control-size) + var(--space-2));
    right: 0;
    min-width: 240px;
    padding: var(--space-2);
    border-radius: var(--radius-md);
    background: var(--panel-strong);
    box-shadow: var(--shadow-popover);
    display: grid;
    gap: var(--space-1);
    z-index: 100;
  }
</style>
