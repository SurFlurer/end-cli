import { asInt, asRecord, asString } from './coercions';
import type { AicDraft } from './types';

export interface DraftStorageConfig {
  draftStorageKey: string;
  paneRatioStorageKey: string;
  minPaneRatio: number;
  maxPaneRatio: number;
}

export interface RestoredLocalState {
  draft: AicDraft | null;
  leftPaneRatio: number | null;
}

function getStorage(): Storage | null {
  if (typeof window === 'undefined') {
    return null;
  }

  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function parseStoredDraft(text: string): AicDraft | null {
  try {
    const root = JSON.parse(text);
    if (typeof root !== 'object' || root === null || Array.isArray(root)) {
      return null;
    }

    const parsed = root as Record<string, unknown>;
    const regionRaw = asString(parsed.region).trim();
    const region = regionRaw === 'fourth_valley' ? 'fourth_valley' : 'wuling';
    const supplyRows = Array.isArray(parsed.supply) ? parsed.supply : [];
    const consumptionRows = Array.isArray(parsed.consumption) ? parsed.consumption : [];
    const outpostRows = Array.isArray(parsed.outposts) ? parsed.outposts : [];

    return {
      region,
      externalPowerConsumptionW: asInt(parsed.externalPowerConsumptionW),
      supply: supplyRows.map((row) => {
        const record = asRecord(row);
        return {
          itemKey: asString(record.itemKey),
          value: asInt(record.value)
        };
      }),
      consumption: consumptionRows.map((row) => {
        const record = asRecord(row);
        return {
          itemKey: asString(record.itemKey),
          value: asInt(record.value)
        };
      }),
      outposts: outpostRows.map((row) => {
        const record = asRecord(row);
        const priceRows = Array.isArray(record.prices) ? record.prices : [];
        const zh = asString(record.zh).trim();
        const en = asString(record.en).trim();
        const name = asString(record.name).trim();
        return {
          key: asString(record.key),
          name: name || zh || en,
          moneyCapPerHour: asInt(record.moneyCapPerHour),
          prices: priceRows.map((priceRow) => {
            const priceRecord = asRecord(priceRow);
            return {
              itemKey: asString(priceRecord.itemKey),
              price: asInt(priceRecord.price)
            };
          })
        };
      })
    };
  } catch {
    return null;
  }
}

export function restoreLocalState(config: DraftStorageConfig): RestoredLocalState {
  const storage = getStorage();
  if (!storage) {
    return {
      draft: null,
      leftPaneRatio: null
    };
  }

  let leftPaneRatio: number | null = null;
  const storedRatio = storage.getItem(config.paneRatioStorageKey);
  if (storedRatio !== null) {
    const nextRatio = Number(storedRatio);
    if (Number.isFinite(nextRatio)) {
      leftPaneRatio = Math.min(config.maxPaneRatio, Math.max(config.minPaneRatio, nextRatio));
    }
  }

  const storedDraft = storage.getItem(config.draftStorageKey);
  if (storedDraft === null) {
    return {
      draft: null,
      leftPaneRatio
    };
  }

  const draft = parseStoredDraft(storedDraft);
  if (!draft) {
    try {
      storage.removeItem(config.draftStorageKey);
    } catch {
      // Ignore write failures in restricted browser modes.
    }
  }

  return {
    draft,
    leftPaneRatio
  };
}

export function persistDraft(storageKey: string, draft: AicDraft): void {
  const storage = getStorage();
  if (!storage) {
    return;
  }

  try {
    storage.setItem(storageKey, JSON.stringify(draft));
  } catch {
    // Ignore write failures in restricted browser modes.
  }
}

export function persistLeftPaneRatio(storageKey: string, ratio: number): void {
  const storage = getStorage();
  if (!storage) {
    return;
  }

  try {
    storage.setItem(storageKey, String(ratio));
  } catch {
    // Ignore write failures in restricted browser modes.
  }
}
