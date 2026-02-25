<script lang="ts">
  import { tooltip, type TooltipValue } from "../lib/tooltip";

  export type DataTableCell =
    | string
    | {
        text: string;
        icon?: string;
        className?: string;
        tooltip?: TooltipValue;
      };

  interface Props {
    title: string;
    headers: string[];
    rows: DataTableCell[][];
    numericColumns?: number[];
  }

  let { title, headers, rows, numericColumns = [] }: Props = $props();

  const numericColumnSet = $derived(new Set(numericColumns));

  function isNumericColumn(index: number): boolean {
    return numericColumnSet.has(index);
  }
</script>

{#if rows.length > 0}
  <div class="table-wrap">
    <h3>{title}</h3>
    <div class="table-scroll">
      <table>
        <thead>
          <tr>
            {#each headers as header, index}
              <th class:numeric={isNumericColumn(index)}>{header}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
            <tr>
              {#each row as value, index}
                <td class:numeric={isNumericColumn(index)}>
                  {#if typeof value === 'string'}
                    {value}
                  {:else}
                    <span
                      class={['cell', value.className ?? ''].filter(Boolean).join(' ')}
                      use:tooltip={value.tooltip}
                    >
                      {#if value.icon}
                        <span class="material-symbols-outlined cell-icon" aria-hidden="true"
                          >{value.icon}</span
                        >
                      {/if}
                      <span>{value.text}</span>
                    </span>
                  {/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
{/if}

<style>
  .table-wrap {
    display: grid;
    gap: var(--space-2);
  }

  .table-scroll {
    background: var(--panel-strong);
    min-width: 0;
    max-width: 100%;
  }

  th:first-child,
  td:first-child {
    padding-left: 12px;
  }

  table {
    width: max-content;
    min-width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }

  th,
  td {
    border-bottom: 1px solid color-mix(in srgb, var(--line) 78%, var(--line-tint-2));
    text-align: left;
    padding: 8px 6px;
    overflow-wrap: anywhere;
  }

  th.numeric,
  td.numeric {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tbody tr:last-child td {
    border-bottom: none;
  }

  td {
    transition: background-color 140ms ease;
  }

  @media (hover: hover) and (pointer: fine) {
    tr:hover {
      background: var(--surface-soft);
    }
  }

  th {
    color: var(--ink-soft);
    font-weight: 600;
  }

  .cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .cell-icon {
    font-size: 16px;
    line-height: 1;
    font-variation-settings:
      "FILL" 0,
      "wght" 600,
      "GRAD" 0,
      "opsz" 16;
  }

  .cell-good {
    color: var(--good, var(--accent));
    font-weight: 700;
  }

  .cell-warn {
    color: var(--warn);
    font-weight: 700;
  }
</style>
