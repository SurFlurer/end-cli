export type OutpostSelection =
  | { kind: 'none' }
  | { kind: 'selected'; index: number };

export const NO_OUTPOST_SELECTED: OutpostSelection = { kind: 'none' };

export function isSameOutpostSelection(a: OutpostSelection, b: OutpostSelection): boolean {
  if (a.kind === 'none') {
    return b.kind === 'none';
  }

  return b.kind === 'selected' && a.index === b.index;
}
