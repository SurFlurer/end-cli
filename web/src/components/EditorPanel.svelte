<script lang="ts">
  import FieldHint from "./FieldHint.svelte";
  import SelectField, { type SelectOption } from "./SelectField.svelte";
  import type { EditorPanelProps } from "../lib/editor-actions";
  import type { OutpostDraft } from "../lib/types";

  let {
    lang,
    draft,
    catalogItems,
    selectedOutpostIndex,
    isResetDisabled,
    actions,
  }: EditorPanelProps = $props();

  const selectedOutpost = $derived<OutpostDraft | null>(
    draft.outposts[selectedOutpostIndex] ?? null,
  );
  const catalogOptions = $derived<SelectOption[]>(
    catalogItems.map((item) => ({
      value: item.key,
      label: t(item.zh, item.en),
    })),
  );
  const regionOptions = $derived<SelectOption[]>([
    { value: "fourth_valley", label: t("四号谷地", "Fourth Valley") },
    { value: "wuling", label: t("武陵", "Wuling") },
  ]);

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }
</script>

<section class="editor-shell">
  <div class="panel-head">
    <div>
      <h2>{t("求解输入", "Solver Inputs")}</h2>
      <p class="subtitle">
        {t(
          "设置供给、外部消耗、据点收购价和电力预算，右侧会自动计算收益方案。",
          "Set supply, external consumption, outpost prices, and power budget. The right side auto-solves revenue plans.",
        )}
      </p>
    </div>
    <div class="panel-actions">
      <button
        class="danger icon-btn"
        onclick={actions.resetToDefault}
        disabled={isResetDisabled}
        aria-label={t("重置默认配置", "Reset Default")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >delete_forever</span
        >
      </button>

      <label
        class="file-btn secondary icon-btn"
        aria-label={t("导入 aic.toml", "Import aic.toml")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >upload</span
        >
        <input
          type="file"
          accept=".toml,text/plain"
          onchange={actions.importFromFile}
        />
      </label>

      <button
        class="secondary icon-btn"
        onclick={actions.exportToml}
        aria-label={t("导出 aic.toml", "Export aic.toml")}
        title={t("导出 aic.toml", "Export aic.toml")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >download</span
        >
      </button>
    </div>
  </div>

  <div class="field-row">
    <div class="label-with-hint">
      <label for="scenario-region">{t("地区", "Region")}</label>
      <FieldHint
        text={t(
          "地区不会改变据点信息；主要影响部分带地区限制的机器是否可用。",
          "Region does not change outpost data; it mainly controls availability for machines with region locks.",
        )}
      />
    </div>
    <SelectField
      value={draft.region}
      options={regionOptions}
      ariaLabel={t("选择地区", "Select region")}
      searchPlaceholder={t("搜索地区...", "Search regions...")}
      emptyText={t("无匹配地区", "No matching regions")}
      onChange={(nextValue) =>
        actions.setRegion(nextValue as "fourth_valley" | "wuling")}
    />
  </div>

  <div class="field-row">
    <div class="label-with-hint">
      <label for="external-power"
        >{t("基地内外额外耗电 (W)", "External Power (W)")}</label
      >
      <FieldHint
        text={t(
          "用于建模矿点、滑索、作战设备，以及基地内未被程序显式建模的其他生产线耗电；使用这个数字和外部供给、外部消耗一起描述系统外影响。",
          "Models power used by mining points, ziplines, combat devices, and other in-base lines not explicitly modeled; together with external supply/consumption, this captures off-model effects.",
        )}
      />
    </div>
    <input
      id="external-power"
      type="number"
      min="0"
      value={draft.externalPowerConsumptionW}
      oninput={(event) => {
        actions.setExternalPower(
          Number((event.currentTarget as HTMLInputElement).value),
        );
      }}
    />
  </div>

  <article class="sub-panel">
    <div class="sub-header">
      <div class="heading-with-hint">
        <h3>{t("外部供给 / min", "External Supply / min")}</h3>
        <FieldHint
          text={t(
            "通常用于表示矿点持续开采带来的矿物供给。",
            "Typically used for minerals continuously supplied by mining points.",
          )}
        />
      </div>
      <button
        class="tiny"
        onclick={actions.supply.add}
        aria-label={t("添加供给条目", "Add supply row")}
        title={t("添加供给条目", "Add supply row")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >add</span
        >
      </button>
    </div>

    {#if draft.supply.length === 0}
      <p class="hint">{t("暂无供给条目。", "No supply rows yet.")}</p>
    {/if}

    {#each draft.supply as row, rowIndex}
      <div class="row-grid">
        <SelectField
          value={row.itemKey}
          options={catalogOptions}
          ariaLabel={t("选择供给物品", "Select supply item")}
          searchPlaceholder={t("搜索物品...", "Search items...")}
          emptyText={t("无匹配物品", "No matching items")}
          onChange={(nextValue) => actions.supply.setKey(rowIndex, nextValue)}
        />

        <input
          type="number"
          min="0"
          value={row.value}
          oninput={(event) =>
            actions.supply.setValue(
              rowIndex,
              Number((event.currentTarget as HTMLInputElement).value),
            )}
        />

        <button
          class="danger tiny row-action"
          onclick={() => actions.supply.remove(rowIndex)}
          aria-label={t("删除供给条目", "Remove supply row")}
          title={t("删除供给条目", "Remove supply row")}
        >
          <span class="material-symbols-outlined icon" aria-hidden="true"
            >close</span
          >
        </button>
      </div>
    {/each}
  </article>

  <article class="sub-panel">
    <div class="sub-header">
      <div class="heading-with-hint">
        <h3>{t("外部消耗 / min", "External Consumption / min")}</h3>
        <FieldHint
          text={t(
            "通常用于表示日用品和快递货物等外部需求，你需要将需求转换成每分钟的平均数据。",
            "Typically used for stable external demand such as daily supplies and delivery cargo, converted to per-minute averages.",
          )}
        />
      </div>
      <button
        class="tiny"
        onclick={actions.consumption.add}
        aria-label={t("添加消耗条目", "Add consumption row")}
        title={t("添加消耗条目", "Add consumption row")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >add</span
        >
      </button>
    </div>

    {#if draft.consumption.length === 0}
      <p class="hint">{t("暂无消耗条目。", "No consumption rows yet.")}</p>
    {/if}

    {#each draft.consumption as row, rowIndex}
      <div class="row-grid">
        <SelectField
          value={row.itemKey}
          options={catalogOptions}
          ariaLabel={t("选择消耗物品", "Select consumption item")}
          searchPlaceholder={t("搜索物品...", "Search items...")}
          emptyText={t("无匹配物品", "No matching items")}
          onChange={(nextValue) =>
            actions.consumption.setKey(rowIndex, nextValue)}
        />

        <input
          type="number"
          min="0"
          value={row.value}
          oninput={(event) =>
            actions.consumption.setValue(
              rowIndex,
              Number((event.currentTarget as HTMLInputElement).value),
            )}
        />

        <button
          class="danger tiny row-action"
          onclick={() => actions.consumption.remove(rowIndex)}
          aria-label={t("删除消耗条目", "Remove consumption row")}
          title={t("删除消耗条目", "Remove consumption row")}
        >
          <span class="material-symbols-outlined icon" aria-hidden="true"
            >close</span
          >
        </button>
      </div>
    {/each}
  </article>

  <article class="sub-panel">
    <div class="sub-header">
      <div class="heading-with-hint">
        <h3>{t("据点与收购价", "Outposts & Buy Prices")}</h3>
        <FieldHint
          text={t(
            "此列表可以留空；填入后求解器会根据收购价和供需情况自动计算生产什么产品来交易以最大化利润。",
            "This section can be left empty for the solver to ignore outposts; when filled, the solver automatically calculates which products to produce for trading based on their prices and supply/demand to maximize profit.",
          )}
        />
      </div>
      <button
        class="tiny"
        onclick={actions.outposts.add}
        aria-label={t("添加据点", "Add outpost")}
        title={t("添加据点", "Add outpost")}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true"
          >add</span
        >
      </button>
    </div>

    {#if draft.outposts.length === 0}
      <p class="hint">{t("暂无据点。", "No outposts yet.")}</p>
    {/if}

    {#if draft.outposts.length > 0}
      <div class="outpost-layout">
        <div class="outpost-list">
          {#each draft.outposts as outpost, outpostIndex}
            <button
              type="button"
              class={`outpost-pick ${outpostIndex === selectedOutpostIndex ? "active" : ""}`}
              onclick={() => actions.outposts.select(outpostIndex)}
            >
              <p class="outpost-pick-title">
                {outpost.name ||
                  outpost.key ||
                  `${t("据点", "Outpost")} ${outpostIndex + 1}`}
              </p>
              <p class="outpost-pick-meta">
                {t("每小时交易上限", "Money Cap / h")}: {outpost.moneyCapPerHour}
              </p>
              <p class="outpost-pick-meta">
                {t("收购条目", "Price rows")}: {outpost.prices.length}
              </p>
            </button>
          {/each}
        </div>

        {#if selectedOutpost}
          <article class="outpost-card">
            <div class="outpost-head">
              <h4>
                {selectedOutpost.name ||
                  selectedOutpost.key ||
                  `${t("据点", "Outpost")} ${(selectedOutpostIndex >= 0 ? selectedOutpostIndex : 0) + 1}`}
              </h4>
              <button
                class="danger tiny"
                onclick={() => actions.outposts.remove(selectedOutpostIndex)}
                aria-label={t("删除据点", "Remove outpost")}
                title={t("删除据点", "Remove outpost")}
              >
                <span class="material-symbols-outlined icon" aria-hidden="true"
                  >close</span
                >
              </button>
            </div>

            <div class="row-grid two">
              <label>
                {t("名称", "Name (Optional)")}
                <input
                  type="text"
                  value={selectedOutpost.name}
                  oninput={(event) =>
                    actions.outposts.setField(
                      selectedOutpostIndex,
                      "name",
                      (event.currentTarget as HTMLInputElement).value,
                    )}
                />
              </label>

              <label>
                <span class="label-with-hint compact">
                  <span>{t("每小时交易上限", "Money Cap / h")}</span>
                  <FieldHint
                    text={t(
                      "据点每小时可用于交易的金额上限。",
                      "Maximum amount of money this outpost can trade per hour.",
                    )}
                  />
                </span>
                <input
                  type="number"
                  min="0"
                  value={selectedOutpost.moneyCapPerHour}
                  oninput={(event) =>
                    actions.outposts.setField(
                      selectedOutpostIndex,
                      "moneyCapPerHour",
                      Number((event.currentTarget as HTMLInputElement).value),
                    )}
                />
              </label>
            </div>

            <div class="sub-header mini">
              <div class="heading-with-hint">
                <h5>{t("收购价", "Buy Prices")}</h5>
                <FieldHint
                  text={t(
                    "除按价格表填写外，可手动删去低价且容易爆仓的条目来收缩优化范围。本程序使用稳态模型，不会主动考虑爆仓风险。",
                    "Besides filling from price table, you can remove low-value, overflow-prone rows to narrow optimization scope. This program uses a steady-state model and does not actively consider overflow risk.",
                  )}
                />
              </div>
              <button
                class="tiny"
                onclick={() => actions.prices.add(selectedOutpostIndex)}
                aria-label={t("添加价格条目", "Add price row")}
                title={t("添加价格条目", "Add price row")}
              >
                <span class="material-symbols-outlined icon" aria-hidden="true"
                  >add</span
                >
              </button>
            </div>

            {#if selectedOutpost.prices.length === 0}
              <p class="hint">{t("暂无价格条目。", "No price rows yet.")}</p>
            {/if}

            {#each selectedOutpost.prices as price, priceIndex}
              <div class="row-grid">
                <SelectField
                  value={price.itemKey}
                  options={catalogOptions}
                  ariaLabel={t("选择收购物品", "Select price item")}
                  searchPlaceholder={t("搜索物品...", "Search items...")}
                  emptyText={t("无匹配物品", "No matching items")}
                  onChange={(nextValue) =>
                    actions.prices.setKey(
                      selectedOutpostIndex,
                      priceIndex,
                      nextValue,
                    )}
                />

                <input
                  type="number"
                  min="0"
                  value={price.price}
                  oninput={(event) =>
                    actions.prices.setValue(
                      selectedOutpostIndex,
                      priceIndex,
                      Number((event.currentTarget as HTMLInputElement).value),
                    )}
                />

                <button
                  class="danger tiny row-action"
                  onclick={() =>
                    actions.prices.remove(selectedOutpostIndex, priceIndex)}
                  aria-label={t("删除价格条目", "Remove price row")}
                  title={t("删除价格条目", "Remove price row")}
                >
                  <span
                    class="material-symbols-outlined icon"
                    aria-hidden="true">close</span
                  >
                </button>
              </div>
            {/each}
          </article>
        {/if}
      </div>
    {/if}
  </article>
</section>

<style>
  .editor-shell {
    display: grid;
    gap: var(--space-3);
  }

  .sub-panel {
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    display: grid;
    gap: var(--space-2);
    background: color-mix(in srgb, var(--panel-strong) 92%, #eef7f2);
  }

  .sub-header.mini {
    margin-top: var(--space-1);
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 190px;
    gap: var(--space-2);
    align-items: center;
  }

  .label-with-hint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .label-with-hint.compact {
    width: fit-content;
  }

  .heading-with-hint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .row-grid {
    display: grid;
    gap: var(--space-2);
    grid-template-columns: 1fr 140px 36px;
  }

  .row-grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .outpost-layout {
    display: grid;
    gap: var(--space-3);
    grid-template-columns: 200px minmax(0, 1fr);
  }

  .outpost-list {
    display: grid;
    gap: var(--space-2);
    align-content: start;
  }

  .outpost-pick {
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--panel-strong);
    display: grid;
    gap: var(--space-1);
    text-align: left;
    padding: var(--space-3);
  }

  .outpost-pick.active {
    border-color: color-mix(in srgb, var(--accent) 58%, #79c2ab);
    background: color-mix(in srgb, var(--accent-soft) 62%, #f9fdfb);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, #cde7dd);
  }

  .outpost-pick-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }

  .outpost-pick-meta {
    margin: 0;
    color: var(--ink-soft);
    font-size: 12px;
  }

  .outpost-card {
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    display: grid;
    gap: var(--space-2);
    background: var(--panel-strong);
    container-type: inline-size;
  }

  .row-grid.two label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .outpost-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-2);
  }

  input {
    border: 1px solid color-mix(in srgb, var(--line) 90%, #b6cec2);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    background: var(--panel-strong);
    color: inherit;
    font: inherit;
    line-height: 1.2;
  }

  button,
  .file-btn {
    border: 1px solid color-mix(in srgb, var(--line) 90%, #b6cec2);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    background: var(--panel-strong);
    color: inherit;
    font: inherit;
    line-height: 1.2;
    cursor: pointer;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .secondary {
    background: var(--surface-soft);
  }

  .file-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: var(--control-size);
  }

  .icon-btn {
    width: var(--control-size);
    height: var(--control-size);
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
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

  .file-btn input {
    display: none;
  }

  .tiny {
    width: var(--control-size);
    height: var(--control-size);
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    flex: 0 0 auto;
  }

  .row-action {
    width: 100%;
  }

  .danger {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 38%, #d7b0bc);
    background: color-mix(in srgb, var(--danger) 8%, #fff);
  }

  .hint {
    margin: 0;
    color: var(--muted-text);
    font-size: 13px;
  }

  @media (max-width: 1200px) {
    .outpost-layout {
      grid-template-columns: 1fr;
    }
  }

  @container (max-width: 480px) {
    .row-grid.two {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 760px) {
    .field-row {
      grid-template-columns: 1fr;
    }

    .row-grid {
      grid-template-columns: 1fr 104px 36px;
    }

    .row-grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>
