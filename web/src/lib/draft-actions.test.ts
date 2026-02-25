import { describe, expect, it } from 'vitest';
import {
  addConsumptionRow,
  addOutpost,
  addPriceRow,
  addSupplyRow,
  normalizeSelectedOutpostIndex,
  removeConsumptionRow,
  removeOutpost,
  removePriceRow,
  removeSupplyRow,
  setConsumptionValue,
  setExternalPower,
  setStage2Objective,
  setStage2Weight,
  setOutpostField,
  setPriceValue,
  setSupplyValue
} from './draft-actions';
import type { AicDraft } from './types';

const EMPTY_DRAFT: AicDraft = {
  region: 'wuling',
  externalPowerConsumptionW: 0,
  stage2: {
    objective: 'min_machines',
    alpha: 1,
    beta: 1,
    gamma: 1
  },
  supply: [],
  consumption: [],
  outposts: []
};

describe('draft actions', () => {
  it('normalizes numeric inputs as non-negative integers', () => {
    let draft = setExternalPower(EMPTY_DRAFT, -12.3);
    draft = addSupplyRow(draft, 'IronOre');
    draft = setSupplyValue(draft, 0, 4.8);
    draft = addConsumptionRow(draft, 'Water');
    draft = setConsumptionValue(draft, 0, 6.7);

    expect(draft.externalPowerConsumptionW).toBe(0);
    expect(draft.supply[0]?.value).toBe(5);
    expect(draft.consumption[0]?.value).toBe(7);
  });

  it('adds and removes outposts while maintaining selected index', () => {
    const added1 = addOutpost(EMPTY_DRAFT);
    const added2 = addOutpost(added1.draft);

    expect(added2.draft.outposts).toHaveLength(2);
    expect(added2.selectedOutpostIndex).toBe(1);
    expect(added2.draft.outposts[0]?.key).toBe('outpost_1');
    expect(added2.draft.outposts[1]?.key).toBe('outpost_2');

    const removed = removeOutpost(added2.draft, added2.selectedOutpostIndex, 1);
    expect(removed.draft.outposts).toHaveLength(1);
    expect(removed.selectedOutpostIndex).toBe(0);
  });

  it('updates outpost fields and price rows immutably', () => {
    const added = addOutpost(EMPTY_DRAFT);

    let draft = setOutpostField(added.draft, 0, 'name', 'TradeHub');
    draft = addPriceRow(draft, 0, 'Circuit');
    draft = setPriceValue(draft, 0, 0, 12.6);

    expect(draft.outposts[0]?.name).toBe('TradeHub');
    expect(draft.outposts[0]?.key).toBe('outpost_1');
    expect(draft.outposts[0]?.prices[0]?.price).toBe(13);

    const afterRemovePrice = removePriceRow(draft, 0, 0);
    expect(afterRemovePrice.outposts[0]?.prices).toHaveLength(0);
  });

  it('removes supply rows by index', () => {
    let draft = addSupplyRow(EMPTY_DRAFT, 'A');
    draft = addSupplyRow(draft, 'B');

    const after = removeSupplyRow(draft, 0);
    expect(after.supply).toHaveLength(1);
    expect(after.supply[0]?.itemKey).toBe('B');
  });

  it('removes consumption rows by index', () => {
    let draft = addConsumptionRow(EMPTY_DRAFT, 'A');
    draft = addConsumptionRow(draft, 'B');

    const after = removeConsumptionRow(draft, 0);
    expect(after.consumption).toHaveLength(1);
    expect(after.consumption[0]?.itemKey).toBe('B');
  });

  it('normalizes selected outpost index for empty and invalid states', () => {
    expect(normalizeSelectedOutpostIndex(EMPTY_DRAFT, 2)).toBe(-1);

    const added = addOutpost(EMPTY_DRAFT);
    expect(normalizeSelectedOutpostIndex(added.draft, 9)).toBe(0);
  });

  it('updates stage2 objective and weighted coefficients', () => {
    let draft = setStage2Objective(EMPTY_DRAFT, 'weighted');
    draft = setStage2Weight(draft, 'alpha', 1.25);
    draft = setStage2Weight(draft, 'beta', 2.5);
    draft = setStage2Weight(draft, 'gamma', -3);

    expect(draft.stage2.objective).toBe('weighted');
    expect(draft.stage2.alpha).toBe(1.25);
    expect(draft.stage2.beta).toBe(2.5);
    expect(draft.stage2.gamma).toBe(0);
  });
});
