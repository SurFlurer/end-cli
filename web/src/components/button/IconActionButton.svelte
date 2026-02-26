<script lang="ts">
  import type { Snippet } from "svelte";
  import { tooltip } from "../../lib/tooltip";

  interface FileInputConfig {
    accept: string;
    onChange: (event: Event) => void;
  }

  interface CommonProps {
    ariaLabel: string;
    icon?: string;
    iconSnippet?: Snippet;
    label?: string;
    title?: string;
    kind?: "default" | "danger";
    disabled?: boolean;
    active?: boolean;
    onClick?: () => void;
    className?: string;
  }

  interface Props extends CommonProps {
    fileInput?: FileInputConfig;
    href?: string;
    target?: string;
    rel?: string;
  }

  let {
    ariaLabel,
    icon,
    iconSnippet,
    label,
    title,
    kind = "default",
    disabled = false,
    active = false,
    onClick,
    className = "",
    fileInput,
    href,
    target,
    rel,
  }: Props = $props();

  const rootClass = $derived.by(() => {
    const classes = ["icon-action-btn"];

    if (label && label.trim().length > 0) {
      classes.push("with-label");
    }

    if (kind === "danger") {
      classes.push("danger");
    }

    if (disabled) {
      classes.push("disabled");
    }

    if (active) {
      classes.push("active");
    }

    if (className.trim().length > 0) {
      classes.push(className.trim());
    }

    return classes.join(" ");
  });
</script>

{#if fileInput}
  <label class={rootClass} aria-label={ariaLabel} use:tooltip={title}>
    {#if iconSnippet}
      {@render iconSnippet()}
    {:else if icon}
      <span class="material-symbols-outlined icon" aria-hidden="true">{icon}</span>
    {/if}
    {#if label}
      <span class="text">{label}</span>
    {/if}
    <input
      type="file"
      accept={fileInput.accept}
      onchange={fileInput.onChange}
      disabled={disabled}
    />
  </label>
{:else if href}
  <a
    class={rootClass}
    href={href}
    target={target}
    rel={rel}
    aria-label={ariaLabel}
    aria-current={active ? "page" : undefined}
    aria-disabled={disabled || undefined}
    tabindex={disabled ? -1 : undefined}
    use:tooltip={title}
    onclick={(event) => {
      if (disabled) {
        event.preventDefault();
        return;
      }
      onClick?.();
    }}
  >
    {#if iconSnippet}
      {@render iconSnippet()}
    {:else if icon}
      <span class="material-symbols-outlined icon" aria-hidden="true">{icon}</span>
    {/if}
    {#if label}
      <span class="text">{label}</span>
    {/if}
  </a>
{:else}
  <button
    type="button"
    class={rootClass}
    aria-label={ariaLabel}
    use:tooltip={title}
    onclick={onClick}
    disabled={disabled}
  >
    {#if iconSnippet}
      {@render iconSnippet()}
    {:else if icon}
      <span class="material-symbols-outlined icon" aria-hidden="true">{icon}</span>
    {/if}
    {#if label}
      <span class="text">{label}</span>
    {/if}
  </button>
{/if}

<style>
  .icon-action-btn {
    border: none;
    border-radius: var(--radius-sm);
    width: var(--control-size);
    height: var(--control-size);
    padding: 0;
    background: none;
    color: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    cursor: pointer;
    flex: 0 0 auto;
    --icon-action-hover-bg: color-mix(in srgb,var(--surface-soft) 60%,var(--accent-soft));
  }

  .icon-action-btn.with-label {
    width: auto;
    padding: 0 10px;
    gap: 6px;
  }

  .icon-action-btn.danger {
    color: var(--danger);
    --icon-action-hover-bg: color-mix(in srgb, var(--danger) 12%, var(--panel-strong));
  }

  .icon-action-btn.disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .icon-action-btn.active {
    background: var(--icon-action-hover-bg);
  }

  a.icon-action-btn {
    text-decoration: none;
  }

  .icon-action-btn input {
    display: none;
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

  .text {
    font-size: 12px;
    line-height: 1;
    white-space: nowrap;
  }

  @media (hover: hover) and (pointer: fine) {
    .icon-action-btn:hover:not(.disabled):not(:disabled) {
      background: var(--icon-action-hover-bg);
    }
  }

  .icon-action-btn:focus-visible {
    background: var(--icon-action-hover-bg);
  }
</style>
