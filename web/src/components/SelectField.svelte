<script lang="ts">
  import { tick } from "svelte";

  export interface SelectOption {
    value: string;
    label: string;
  }

  interface Props {
    value: string;
    options: SelectOption[];
    onChange: (nextValue: string) => void;
    ariaLabel: string;
    searchPlaceholder?: string;
    emptyText?: string;
    searchable?: boolean;
    disabled?: boolean;
  }

  let {
    value,
    options,
    onChange,
    ariaLabel,
    searchPlaceholder = "",
    emptyText = "",
    searchable = true,
    disabled = false,
  }: Props = $props();

  let rootElement = $state<HTMLElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  let isOpen = $state(false);
  let query = $state("");

  const normalizedQuery = $derived(query.trim().toLocaleLowerCase());
  const resolvedSearchPlaceholder = $derived(searchPlaceholder || ariaLabel);
  const searchInputAriaLabel = $derived(
    `${ariaLabel} ${searchPlaceholder || "search"}`,
  );
  const selectedOption = $derived(
    options.find((option) => option.value === value) ?? null,
  );
  const filteredOptions = $derived.by(() => {
    if (!searchable) {
      return options;
    }

    if (normalizedQuery.length === 0) {
      return options;
    }

    return options.filter((option) =>
      option.label.toLocaleLowerCase().includes(normalizedQuery),
    );
  });

  function closePanel(): void {
    if (!isOpen) {
      return;
    }
    isOpen = false;
    query = "";
  }

  async function openPanel(): Promise<void> {
    if (disabled || isOpen) {
      return;
    }

    isOpen = true;
    query = "";
    await tick();
    if (searchable) {
      searchInput?.focus();
    }
  }

  function togglePanel(): void {
    if (isOpen) {
      closePanel();
      return;
    }
    void openPanel();
  }

  function commitSelection(nextValue: string): void {
    if (nextValue !== value) {
      onChange(nextValue);
    }
    closePanel();
  }

  $effect(() => {
    if (!isOpen || typeof document === "undefined") {
      return;
    }

    const onPointerDown = (event: PointerEvent): void => {
      const target = event.target;
      if (
        rootElement &&
        target instanceof Node &&
        !rootElement.contains(target)
      ) {
        closePanel();
      }
    };
    const onWindowBlur = (): void => {
      closePanel();
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") {
        return;
      }
      event.preventDefault();
      closePanel();
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onWindowBlur);

    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", onWindowBlur);
    };
  });
</script>

<div class={`select-field ${isOpen ? "is-open" : ""}`} bind:this={rootElement}>
  <button
    type="button"
    class="trigger"
    onclick={togglePanel}
    {disabled}
    aria-label={ariaLabel}
    aria-expanded={isOpen}
    aria-haspopup="listbox"
  >
    <span class="trigger-label">{selectedOption?.label ?? value}</span>
    <span class="chevron" aria-hidden="true"></span>
  </button>

  {#if isOpen}
    <div class="menu" role="dialog" aria-label={ariaLabel}>
      {#if searchable}
        <input
          class="search"
          bind:this={searchInput}
          type="search"
          value={query}
          placeholder={resolvedSearchPlaceholder}
          aria-label={searchInputAriaLabel}
          oninput={(event) => {
            query = (event.currentTarget as HTMLInputElement).value;
          }}
        />
      {/if}

      <div class="options" role="listbox" aria-label={ariaLabel}>
        {#if filteredOptions.length === 0}
          <p class="empty">{emptyText}</p>
        {:else}
          {#each filteredOptions as option (option.value)}
            <button
              type="button"
              class={`option ${option.value === value ? "is-selected" : ""}`}
              role="option"
              aria-selected={option.value === value}
              onclick={() => {
                commitSelection(option.value);
              }}
            >
              {option.label}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .select-field {
    position: relative;
    min-width: 0;
  }

  .trigger {
    width: 100%;
    height: var(--control-size);
    min-height: var(--control-size);
    border: 1px solid color-mix(in srgb, var(--line) 88%, #8cb6a4);
    border-radius: var(--radius-sm);
    padding: 0 12px;
    background: #ffffff;
    color: inherit;
    font: inherit;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    cursor: pointer;
  }

  @media (hover: hover) and (pointer: fine) {
    .trigger:hover {
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 75%, #d8efe5);
    }
  }

  .trigger:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 75%, #d8efe5);
  }

  .trigger:disabled {
    cursor: not-allowed;
    opacity: 0.65;
  }

  .trigger-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .chevron {
    width: 8px;
    height: 8px;
    border-right: 2px solid color-mix(in srgb, var(--ink) 70%, #6f8f83);
    border-bottom: 2px solid color-mix(in srgb, var(--ink) 70%, #6f8f83);
    transform: rotate(45deg);
    transition: transform 240ms ease;
    flex: 0 0 auto;
    margin-top: -2px;
  }

  .is-open .chevron {
    transform: rotate(-135deg);
    margin-top: 3px;
  }

  .menu {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 6px);
    z-index: 25;
    border-radius: var(--radius-md);
    background: #fbfffc;
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.08),
      0 10px 24px rgba(0, 0, 0, 0.1);
    padding: 8px;
    display: grid;
    gap: 8px;
  }

  .search {
    width: 100%;
    border: 1px solid color-mix(in srgb, var(--line) 88%, #96bbaa);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    background: #fff;
    color: inherit;
    font: inherit;
  }

  .search:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 75%, #d8efe5);
  }

  .options {
    max-height: 248px;
    overflow: auto;
    padding-right: 2px;
    display: grid;
    gap: 6px;
  }

  .option {
    width: 100%;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: #fff;
    color: inherit;
    text-align: left;
    font: inherit;
    padding: 6px 10px;
    cursor: pointer;
  }

  .option:hover {
    background: color-mix(in srgb, var(--accent-soft) 62%, #f4fcf8);
  }

  .option.is-selected {
    background: color-mix(in srgb, var(--accent-soft) 74%, #effaf4);
  }

  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--muted-text);
    padding: 6px 4px;
  }
</style>
