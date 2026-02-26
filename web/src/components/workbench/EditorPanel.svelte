<script lang="ts">
  import FieldHint from "../hover/FieldHint.svelte";
  import DropdownMenu from "../hover/DropdownMenu.svelte";
  import IconActionButton from "../button/IconActionButton.svelte";
  import InputField from "../input/InputField.svelte";
  import Panel from "../pane/Panel.svelte";
  import PanelHeader from "../pane/PanelHeader.svelte";
  import SelectField from "../input/SelectField.svelte";
  import type { EditorPanelProps } from "../../lib/editor-actions";
  import type { OutpostDraft } from "../../lib/types";

  type SelectOption = {
    value: string;
    label: string;
  };

  let {
    lang,
    draft,
    catalogItems,
    selectedOutpostIndex,
    isResetDisabled,
    actions,
    onOpenShare,
  }: EditorPanelProps = $props();

  const selectedIndex = $derived<number | null>(
    selectedOutpostIndex.kind === "selected"
      ? selectedOutpostIndex.index
      : null,
  );
  const selectedOutpost = $derived<OutpostDraft | null>(
    selectedIndex === null ? null : (draft.outposts[selectedIndex] ?? null),
  );
  const selectedOutpostOrdinal = $derived(
    selectedOutpostIndex.kind === "selected"
      ? selectedOutpostIndex.index + 1
      : 1,
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
  const stage2ObjectiveOptions = $derived<SelectOption[]>([
    { value: "min_machines", label: t("最少机器", "Min Machines") },
    { value: "max_power_slack", label: t("最大电力余量", "Max Power Slack") },
    { value: "max_money_slack", label: t("最大虚拟成交额", "Max Money Slack") },
    { value: "weighted", label: t("加权目标", "Weighted") },
  ]);

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }
</script>

<Panel>
  {#snippet header()}
    <PanelHeader
      titleText={t("求解输入", "Solver Inputs")}
      subtitleText={t(
        "这里填写输入内容。",
        "Set supply, external consumption, outpost prices, and power budget. The right side auto-solves revenue plans.",
      )}
    >
      {#snippet controls()}
        <IconActionButton
          icon="share"
          onClick={onOpenShare}
          title={t("分享", "Share")}
          ariaLabel={t("分享", "Share")}
        />

        <DropdownMenu
          menuAriaLabel={t("输入操作菜单", "Input actions menu")}
          triggerAriaLabel={t("打开菜单", "Open menu")}
          triggerTitle={t("菜单", "Menu")}
          triggerIcon="more_vert"
          disabled={false}
        >
          {#snippet menu(close)}
            <IconActionButton
              kind="danger"
              icon="delete"
              label={t("重置为示例输入", "Reset to Example Input")}
              onClick={() => {
                close();
                actions.resetToDefault();
              }}
              disabled={isResetDisabled}
              ariaLabel={t("重置示例输入", "Reset Example Input")}
            />

            <IconActionButton
              icon="download"
              label={t("导入 aic.toml", "Import aic.toml")}
              ariaLabel={t("导入 aic.toml", "Import aic.toml")}
              fileInput={{
                accept: ".toml,text/plain",
                onChange: (event) => {
                  close();
                  return actions.importFromFile(event);
                },
              }}
            />

            <IconActionButton
              icon="upload"
              label={t("导出 aic.toml", "Export aic.toml")}
              onClick={() => {
                close();
                actions.exportToml();
              }}
              title={t("导出", "Export")}
              ariaLabel={t("导出 aic.toml", "Export aic.toml")}
            />
          {/snippet}
        </DropdownMenu>
      {/snippet}
    </PanelHeader>
  {/snippet}

  <section class="editor-shell">
    <section>
      <div class="field-row">
        <div class="label-with-hint">
          <label for="stage2-objective">{t("优化目标", "Objective")}</label>
          <FieldHint
            text={t(
              "求解器首先会尝试最大化收益，若有平局，则额外优化该目标。",
              "The solver first maximizes profit, then optimizes the objective set here as a tiebreaker among equally profitable solutions.",
            )}
          />
        </div>
        <SelectField
          id="stage2-objective"
          value={draft.stage2.objective}
          options={stage2ObjectiveOptions}
          ariaLabel={t("选择 Stage2 目标", "Select Stage2 objective")}
          searchable={false}
          onChange={(nextValue) =>
            actions.setStage2Objective(
              nextValue as
                | "min_machines"
                | "max_power_slack"
                | "max_money_slack"
                | "weighted",
            )}
        />
      </div>

      {#if draft.stage2.objective === "weighted"}
        <div class="row-grid three">
          <label>
            <span class="weight-label">α</span>
            <InputField
              type="number"
              min="0"
              step="0.1"
              value={draft.stage2.alpha}
              oninput={(event) =>
                actions.setStage2Weight(
                  "alpha",
                  Number((event.currentTarget as HTMLInputElement).value),
                )}
            />
          </label>
          <label>
            <span class="weight-label">β</span>
            <InputField
              type="number"
              min="0"
              step="0.1"
              value={draft.stage2.beta}
              oninput={(event) =>
                actions.setStage2Weight(
                  "beta",
                  Number((event.currentTarget as HTMLInputElement).value),
                )}
            />
          </label>
          <label>
            <span class="weight-label">γ</span>
            <InputField
              type="number"
              min="0"
              step="0.1"
              value={draft.stage2.gamma}
              oninput={(event) =>
                actions.setStage2Weight(
                  "gamma",
                  Number((event.currentTarget as HTMLInputElement).value),
                )}
            />
          </label>
        </div>
      {/if}

      <div class="field-row">
        <div class="label-with-hint">
          <label for="region">{t("地区", "Region")}</label>
          <FieldHint
            text={t(
              "地区不会改变据点信息；主要影响部分带地区限制的机器是否可用。",
              "Region does not change outpost data; it mainly controls availability for machines with region locks.",
            )}
          />
        </div>
        <SelectField
          id="region"
          value={draft.region}
          options={regionOptions}
          ariaLabel={t("选择地区", "Select region")}
          searchable={false}
          onChange={(nextValue) =>
            actions.setRegion(nextValue as "fourth_valley" | "wuling")}
        />
      </div>

      <div class="field-row">
        <div class="label-with-hint">
          <label for="external-power"
            >{t("外部耗电", "External Power (W)")}</label
          >
          <FieldHint
            text={t(
              "用于建模矿点、滑索、作战设备等基地外部设备耗电，以及基地内未被程序显式建模的其他生产线耗电；使用这个数字和外部供给、外部消耗一起描述系统外影响。",
              "Models power used by mining points, ziplines, combat devices, and other in-base lines not explicitly modeled; together with external supply/consumption, this captures off-model effects.",
            )}
          />
        </div>
        <InputField
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
    </section>

    <section>
      <PanelHeader>
        {#snippet title()}
          <div class="heading-with-hint">
            <h3>{t("外部供给 / min", "External Supply / min")}</h3>
            <FieldHint
              text={t(
                "通常用于表示矿点持续开采的矿物供给。；使用这个数字和外部耗电、外部消耗一起描述系统外影响。",
                "Typically used for minerals continuously supplied by mining points.",
              )}
            />
          </div>
        {/snippet}

        {#snippet controls()}
          <IconActionButton
            icon="add"
            onClick={actions.supply.add}
            ariaLabel={t("添加供给条目", "Add supply row")}
          />
        {/snippet}
      </PanelHeader>

      {#if draft.supply.length === 0}
        <p class="hint">{t("暂无供给条目。", "No supply rows yet.")}</p>
      {/if}

      {#each draft.supply as row, rowIndex (rowIndex)}
        <div class="row-grid">
          <SelectField
            value={row.itemKey}
            options={catalogOptions}
            ariaLabel={t("选择供给物品", "Select supply item")}
            searchPlaceholder={t("搜索物品...", "Search items...")}
            emptyText={t("无匹配物品", "No matching items")}
            onChange={(nextValue) => actions.supply.setKey(rowIndex, nextValue)}
          />

          <InputField
            type="number"
            min="0"
            value={row.value}
            oninput={(event) =>
              actions.supply.setValue(
                rowIndex,
                Number((event.currentTarget as HTMLInputElement).value),
              )}
          />

          <IconActionButton
            icon="horizontal_rule"
            onClick={() => actions.supply.remove(rowIndex)}
            ariaLabel={t("删除供给条目", "Remove supply row")}
          />
        </div>
      {/each}
    </section>

    <section>
      <PanelHeader>
        {#snippet title()}
          <div class="heading-with-hint">
            <h3>{t("外部消耗 / min", "External Consumption / min")}</h3>
            <FieldHint
              text={t(
                "通常用于表示日用品和快递货物等外部需求，须填写每分钟平均数据。",
                "Typically used for stable external demand such as daily supplies and delivery cargo, converted to per-minute averages.",
              )}
            />
          </div>
        {/snippet}

        {#snippet controls()}
          <IconActionButton
            icon="add"
            onClick={actions.consumption.add}
            ariaLabel={t("添加消耗条目", "Add consumption row")}
          />
        {/snippet}
      </PanelHeader>

      {#if draft.consumption.length === 0}
        <p class="hint">{t("暂无消耗条目。", "No consumption rows yet.")}</p>
      {/if}

      {#each draft.consumption as row, rowIndex (rowIndex)}
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

          <InputField
            type="number"
            min="0"
            value={row.value}
            oninput={(event) =>
              actions.consumption.setValue(
                rowIndex,
                Number((event.currentTarget as HTMLInputElement).value),
              )}
          />

          <IconActionButton
            icon="horizontal_rule"
            onClick={() => actions.consumption.remove(rowIndex)}
            ariaLabel={t("删除消耗条目", "Remove consumption row")}
          />
        </div>
      {/each}
    </section>

    <section>
      <PanelHeader>
        {#snippet title()}
          <div class="heading-with-hint">
            <h3>{t("据点", "Outposts")}</h3>
            <FieldHint
              text={t(
                "此列表可以留空；填入后求解器会根据收购价和供需情况自动计算生产什么产品来交易以最大化利润。",
                "This section can be left empty for the solver to ignore outposts; when filled, the solver automatically calculates which products to produce for trading based on their prices and supply/demand to maximize profit.",
              )}
            />
          </div>
        {/snippet}

        {#snippet controls()}
          <IconActionButton
            icon="add"
            onClick={actions.outposts.add}
            ariaLabel={t("添加据点", "Add outpost")}
          />
        {/snippet}
      </PanelHeader>

      {#if draft.outposts.length === 0}
        <p class="hint">{t("暂无据点。", "No outposts yet.")}</p>
      {/if}

      {#if draft.outposts.length > 0}
        <div class="outpost-layout">
          <div class="outpost-list">
            {#each draft.outposts as outpost, outpostIndex (outpost.key)}
              <button
                type="button"
                class={`outpost-pick ${selectedOutpostIndex.kind === "selected" && outpostIndex === selectedOutpostIndex.index ? "active" : ""}`}
                onclick={() => actions.outposts.select(outpostIndex)}
              >
                <p class="outpost-pick-title">
                  {outpost.name ||
                    outpost.key ||
                    `${t("据点", "Outpost")} ${outpostIndex + 1}`}
                </p>
                <p class="outpost-pick-meta">
                  {t("交易上限", "Cap / h")}: {outpost.moneyCapPerHour}
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
                    `${t("据点", "Outpost")} ${selectedOutpostOrdinal}`}
                </h4>
                <IconActionButton
                  icon="delete"
                  onClick={() => {
                    if (selectedOutpostIndex.kind !== "selected") {
                      return;
                    }
                    actions.outposts.remove(selectedOutpostIndex.index);
                  }}
                  ariaLabel={t("删除据点", "Remove outpost")}
                />
              </div>

              <div class="row-grid two">
                <label>
                  {t("名称", "Name (Optional)")}
                  <InputField
                    type="text"
                    value={selectedOutpost.name}
                    oninput={(event) => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.outposts.setField(
                        selectedOutpostIndex.index,
                        "name",
                        (event.currentTarget as HTMLInputElement).value,
                      );
                    }}
                  />
                </label>

                <label>
                  <span class="label-with-hint">
                    <span>{t("每小时交易上限", "Money Cap / h")}</span>
                    <FieldHint
                      text={t(
                        "据点每小时可用于交易的金额上限。",
                        "Maximum amount of money this outpost can trade per hour.",
                      )}
                    />
                  </span>
                  <InputField
                    type="number"
                    min="0"
                    value={selectedOutpost.moneyCapPerHour}
                    oninput={(event) => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.outposts.setField(
                        selectedOutpostIndex.index,
                        "moneyCapPerHour",
                        Number((event.currentTarget as HTMLInputElement).value),
                      );
                    }}
                  />
                </label>
              </div>

              <PanelHeader>
                {#snippet title()}
                  <div class="heading-with-hint">
                    <h5>{t("收购价", "Buy Prices")}</h5>
                    <FieldHint
                      text={t(
                        "除按游戏内价格填写外，可手动删去低价且容易爆仓的条目来收缩优化范围。本程序使用稳态模型，不会主动考虑爆仓风险。",
                        "Besides filling from in-game prices, you can remove low-value, overflow-prone rows to narrow optimization scope. This program uses a steady-state model and does not actively consider overflow risk.",
                      )}
                    />
                  </div>
                {/snippet}

                {#snippet controls()}
                  <IconActionButton
                    icon="add"
                    onClick={() => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.prices.add(selectedOutpostIndex.index);
                    }}
                    ariaLabel={t("添加价格条目", "Add price row")}
                  />
                {/snippet}
              </PanelHeader>

              {#if selectedOutpost.prices.length === 0}
                <p class="hint">{t("暂无价格条目。", "No price rows yet.")}</p>
              {/if}

              {#each selectedOutpost.prices as price, priceIndex (priceIndex)}
                <div class="row-grid">
                  <SelectField
                    value={price.itemKey}
                    options={catalogOptions}
                    ariaLabel={t("选择收购物品", "Select price item")}
                    searchPlaceholder={t("搜索物品...", "Search items...")}
                    emptyText={t("无匹配物品", "No matching items")}
                    onChange={(nextValue) => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.prices.setKey(
                        selectedOutpostIndex.index,
                        priceIndex,
                        nextValue,
                      );
                    }}
                  />

                  <InputField
                    type="number"
                    min="0"
                    value={price.price}
                    oninput={(event) => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.prices.setValue(
                        selectedOutpostIndex.index,
                        priceIndex,
                        Number((event.currentTarget as HTMLInputElement).value),
                      );
                    }}
                  />

                  <IconActionButton
                    icon="horizontal_rule"
                    onClick={() => {
                      if (selectedOutpostIndex.kind !== "selected") {
                        return;
                      }
                      actions.prices.remove(
                        selectedOutpostIndex.index,
                        priceIndex,
                      );
                    }}
                    ariaLabel={t("删除价格条目", "Remove price row")}
                  />
                </div>
              {/each}
            </article>
          {/if}
        </div>
      {/if}
    </section>
  </section>
</Panel>

<style>
  .editor-shell {
    display: grid;
    gap: var(--space-3);
    container-type: inline-size;
  }

  .editor-shell > section {
    display: grid;
    gap: var(--space-2);
  }

  .editor-shell h3 {
    margin: 0;
    line-height: 1.2;
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
  }

  .heading-with-hint {
    display: inline-flex;
    align-items: center;
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
    border-color: color-mix(in srgb, var(--accent) 58%, var(--accent-tint-1));
    box-shadow: inset 0 0 0 2px
      color-mix(in srgb, var(--accent) 35%, var(--accent-tint-2));
  }

  .outpost-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-2);
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

  .row-grid.three label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .row-grid {
    display: grid;
    gap: var(--space-2);
    grid-template-columns: 4fr minmax(64px, 1fr) 36px;
  }

  .row-grid > * {
    min-width: 0;
  }

  .row-grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .row-grid.three {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .outpost-layout {
    display: grid;
    gap: var(--space-3);
  }

  .outpost-list {
    display: grid;
    gap: var(--space-2);
    align-content: start;
  }

  @container (min-width: 480px) {
    .outpost-layout {
      grid-template-columns: minmax(140px, 1fr) minmax(0, 3fr);
    }
  }

  @container (max-width: 480px) {
    .outpost-layout {
      grid-template-columns: 1fr;
    }

    .outpost-list {
      grid-template-columns: 1fr 1fr;
    }
  }

  @container (max-width: 280px) {
    .row-grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>
