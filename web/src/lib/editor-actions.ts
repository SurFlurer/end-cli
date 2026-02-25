import type { AicDraft, CatalogItemDto } from './types';
import type { OutpostSelection } from './outpost-selection';

export type OutpostField = 'name' | 'moneyCapPerHour';
export type Stage2WeightField = 'alpha' | 'beta' | 'gamma';

export interface EditorActions {
  resetToDefault: () => void;
  importFromFile: (event: Event) => void | Promise<void>;
  exportToml: () => void;
  setRegion: (region: 'fourth_valley' | 'wuling') => void;
  setExternalPower: (value: number) => void;
  setStage2Objective: (objective: 'min_machines' | 'max_power_slack' | 'max_money_slack' | 'weighted') => void;
  setStage2Weight: (field: Stage2WeightField, value: number) => void;
  supply: {
    add: () => void;
    remove: (index: number) => void;
    setKey: (index: number, key: string) => void;
    setValue: (index: number, value: number) => void;
  };
  consumption: {
    add: () => void;
    remove: (index: number) => void;
    setKey: (index: number, key: string) => void;
    setValue: (index: number, value: number) => void;
  };
  outposts: {
    add: () => void;
    remove: (index: number) => void;
    select: (index: number) => void;
    setField: (index: number, field: OutpostField, value: string | number) => void;
  };
  prices: {
    add: (outpostIndex: number) => void;
    remove: (outpostIndex: number, priceIndex: number) => void;
    setKey: (outpostIndex: number, priceIndex: number, key: string) => void;
    setValue: (outpostIndex: number, priceIndex: number, value: number) => void;
  };
}

export interface EditorPanelProps {
  lang: 'zh' | 'en';
  draft: AicDraft;
  catalogItems: CatalogItemDto[];
  selectedOutpostIndex: OutpostSelection;
  isResetDisabled: boolean;
  actions: EditorActions;
}
