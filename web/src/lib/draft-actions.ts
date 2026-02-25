import type { AicDraft, OutpostDraft } from './types';
import type { OutpostField } from './editor-actions';

function asNonNegativeInt(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function asNonNegativeNumber(value: number): number {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function nextOutpostKey(draft: AicDraft): string {
  const used = new Set(draft.outposts.map((outpost) => outpost.key.trim().toLowerCase()));
  let index = 1;
  while (used.has(`outpost_${index}`)) {
    index += 1;
  }
  return `outpost_${index}`;
}

function withOutpost(draft: AicDraft, index: number, updater: (outpost: OutpostDraft) => OutpostDraft): AicDraft {
  return {
    ...draft,
    outposts: draft.outposts.map((outpost, outpostIndex) =>
      outpostIndex === index ? updater(outpost) : outpost
    )
  };
}

export function setExternalPower(draft: AicDraft, value: number): AicDraft {
  return {
    ...draft,
    externalPowerConsumptionW: asNonNegativeInt(value)
  };
}

export function setRegion(draft: AicDraft, region: 'fourth_valley' | 'wuling'): AicDraft {
  return {
    ...draft,
    region
  };
}

export function setStage2Objective(
  draft: AicDraft,
  objective: 'min_machines' | 'max_power_slack' | 'max_money_slack' | 'weighted'
): AicDraft {
  return {
    ...draft,
    stage2: {
      ...draft.stage2,
      objective
    }
  };
}

export function setStage2Weight(draft: AicDraft, field: 'alpha' | 'beta' | 'gamma', value: number): AicDraft {
  return {
    ...draft,
    stage2: {
      ...draft.stage2,
      [field]: asNonNegativeNumber(value)
    }
  };
}

export function setSupplyKey(draft: AicDraft, index: number, value: string): AicDraft {
  return {
    ...draft,
    supply: draft.supply.map((row, rowIndex) =>
      rowIndex === index
        ? {
            ...row,
            itemKey: value
          }
        : row
    )
  };
}

export function setSupplyValue(draft: AicDraft, index: number, value: number): AicDraft {
  return {
    ...draft,
    supply: draft.supply.map((row, rowIndex) =>
      rowIndex === index
        ? {
            ...row,
            value: asNonNegativeInt(value)
          }
        : row
    )
  };
}

export function setConsumptionKey(draft: AicDraft, index: number, value: string): AicDraft {
  return {
    ...draft,
    consumption: draft.consumption.map((row, rowIndex) =>
      rowIndex === index
        ? {
            ...row,
            itemKey: value
          }
        : row
    )
  };
}

export function setConsumptionValue(draft: AicDraft, index: number, value: number): AicDraft {
  return {
    ...draft,
    consumption: draft.consumption.map((row, rowIndex) =>
      rowIndex === index
        ? {
            ...row,
            value: asNonNegativeInt(value)
          }
        : row
    )
  };
}

export function addSupplyRow(draft: AicDraft, firstItemKey: string): AicDraft {
  return {
    ...draft,
    supply: [...draft.supply, { itemKey: firstItemKey, value: 1 }]
  };
}

export function addConsumptionRow(draft: AicDraft, firstItemKey: string): AicDraft {
  return {
    ...draft,
    consumption: [...draft.consumption, { itemKey: firstItemKey, value: 1 }]
  };
}

export function removeSupplyRow(draft: AicDraft, index: number): AicDraft {
  return {
    ...draft,
    supply: draft.supply.filter((_, rowIndex) => rowIndex !== index)
  };
}

export function removeConsumptionRow(draft: AicDraft, index: number): AicDraft {
  return {
    ...draft,
    consumption: draft.consumption.filter((_, rowIndex) => rowIndex !== index)
  };
}

export function createOutpost(key: string): OutpostDraft {
  return {
    key,
    name: '',
    moneyCapPerHour: 0,
    prices: []
  };
}

export function addOutpost(
  draft: AicDraft
): { draft: AicDraft; selectedOutpostIndex: number } {
  const outposts = [...draft.outposts, createOutpost(nextOutpostKey(draft))];
  return {
    draft: {
      ...draft,
      outposts
    },
    selectedOutpostIndex: outposts.length - 1
  };
}

export function removeOutpost(
  draft: AicDraft,
  selectedOutpostIndex: number,
  index: number
): { draft: AicDraft; selectedOutpostIndex: number } {
  const outposts = draft.outposts.filter((_, outpostIndex) => outpostIndex !== index);

  let nextSelectedOutpostIndex = selectedOutpostIndex;
  if (outposts.length === 0) {
    nextSelectedOutpostIndex = -1;
  } else if (selectedOutpostIndex > index) {
    nextSelectedOutpostIndex = selectedOutpostIndex - 1;
  } else if (selectedOutpostIndex === index) {
    nextSelectedOutpostIndex = Math.min(index, outposts.length - 1);
  }

  return {
    draft: {
      ...draft,
      outposts
    },
    selectedOutpostIndex: nextSelectedOutpostIndex
  };
}

export function setOutpostField(
  draft: AicDraft,
  index: number,
  field: OutpostField,
  value: string | number
): AicDraft {
  return withOutpost(draft, index, (outpost) => {
    if (field === 'moneyCapPerHour') {
      return {
        ...outpost,
        moneyCapPerHour: typeof value === 'number' ? asNonNegativeInt(value) : 0
      };
    }

    return {
      ...outpost,
      name: String(value)
    };
  });
}

export function addPriceRow(draft: AicDraft, outpostIndex: number, firstItemKey: string): AicDraft {
  return withOutpost(draft, outpostIndex, (outpost) => ({
    ...outpost,
    prices: [...outpost.prices, { itemKey: firstItemKey, price: 0 }]
  }));
}

export function removePriceRow(draft: AicDraft, outpostIndex: number, priceIndex: number): AicDraft {
  return withOutpost(draft, outpostIndex, (outpost) => ({
    ...outpost,
    prices: outpost.prices.filter((_, rowIndex) => rowIndex !== priceIndex)
  }));
}

export function setPriceKey(
  draft: AicDraft,
  outpostIndex: number,
  priceIndex: number,
  itemKey: string
): AicDraft {
  return withOutpost(draft, outpostIndex, (outpost) => ({
    ...outpost,
    prices: outpost.prices.map((row, rowIndex) =>
      rowIndex === priceIndex
        ? {
            ...row,
            itemKey
          }
        : row
    )
  }));
}

export function setPriceValue(
  draft: AicDraft,
  outpostIndex: number,
  priceIndex: number,
  value: number
): AicDraft {
  return withOutpost(draft, outpostIndex, (outpost) => ({
    ...outpost,
    prices: outpost.prices.map((row, rowIndex) =>
      rowIndex === priceIndex
        ? {
            ...row,
            price: asNonNegativeInt(value)
          }
        : row
    )
  }));
}

export function normalizeSelectedOutpostIndex(draft: AicDraft, selectedOutpostIndex: number): number {
  if (draft.outposts.length === 0) {
    return -1;
  }

  if (selectedOutpostIndex < 0 || selectedOutpostIndex >= draft.outposts.length) {
    return 0;
  }

  return selectedOutpostIndex;
}
