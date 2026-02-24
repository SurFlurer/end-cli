<script lang="ts">
  interface Props {
    title: string;
    headers: string[];
    rows: string[][];
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
                <td class:numeric={isNumericColumn(index)}>{value}</td>
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
    border-bottom: 1px solid color-mix(in srgb, var(--line) 78%, #d7e5de);
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
</style>
