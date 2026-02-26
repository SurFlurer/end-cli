export interface HighlightEdgeLabelData extends Record<string, unknown> {
  kind: 'flow-highlight-label';
  topLine: string;
  mainRateText: string;
  mutedRateText?: string;
}

export function isHighlightEdgeLabelData(value: unknown): value is HighlightEdgeLabelData {
  if (typeof value !== 'object' || value === null) {
    return false;
  }

  const maybe = value as Partial<HighlightEdgeLabelData>;
  return (
    maybe.kind === 'flow-highlight-label' &&
    typeof maybe.topLine === 'string' &&
    typeof maybe.mainRateText === 'string' &&
    (maybe.mutedRateText === undefined || typeof maybe.mutedRateText === 'string')
  );
}
