import { describe, expect, it } from 'vitest';
import { reduceSolveState } from './solve-state';
import type { SolveState } from './solve-state';
import type { SolvePayload } from './types';

function mockPayload(): SolvePayload {
  return {
    reportText: 'ok',
    summary: {
      lang: 'en',
      stage1RevenuePerMin: 0,
      stage2RevenuePerMin: 1,
      stage2RevenuePerHour: 60,
      totalMachines: 0,
      totalThermalBanks: 0,
      powerGenW: 0,
      powerUseW: 0,
      powerMarginW: 0,
      outposts: [],
      topSales: [],
      facilities: [],
      externalSupplySlack: []
    },
    logisticsGraph: {
      items: [],
      nodes: [],
      edges: []
    }
  };
}

describe('solve-machine reducer', () => {
  it('debounceScheduled captures previous ok', () => {
    const payload = mockPayload();
    const prev: SolveState = { status: 'ok', payload, elapsedMs: 12 };
    const next = reduceSolveState(prev, { type: 'debounceScheduled' });

    expect(next.status).toBe('pending');
    if (next.status !== 'pending') throw new Error('unreachable');
    expect(next.previousOk?.status).toBe('ok');
    expect(next.previousOk?.payload).toBe(payload);
  });

  it('debounceCleared only affects pending', () => {
    const prevIdle: SolveState = { status: 'idle' };
    expect(reduceSolveState(prevIdle, { type: 'debounceCleared' })).toBe(prevIdle);

    const prevPending: SolveState = { status: 'pending', previousOk: null };
    const next = reduceSolveState(prevPending, { type: 'debounceCleared' });
    expect(next).toEqual({ status: 'idle' });
  });

  it('solveStarted always enters solving and keeps renderedOk', () => {
    const payload = mockPayload();
    const prev: SolveState = { status: 'ok', payload, elapsedMs: 1 };
    const next = reduceSolveState(prev, { type: 'solveStarted', startedAt: 100 });

    expect(next.status).toBe('solving');
    if (next.status !== 'solving') throw new Error('unreachable');
    expect(next.startedAt).toBe(100);
    expect(next.previousOk?.status).toBe('ok');
  });

  it('solveErr trims message and can clear error', () => {
    const prev: SolveState = { status: 'idle' };
    const err = reduceSolveState(prev, { type: 'solveErr', message: '  boom  ' });
    expect(err).toEqual({ status: 'err', message: 'boom' });

    const cleared = reduceSolveState(err, { type: 'clearError' });
    expect(cleared).toEqual({ status: 'idle' });
  });

  it('solveOk sets ok state', () => {
    const payload = mockPayload();
    const prev: SolveState = { status: 'solving', startedAt: 0, previousOk: null };
    const next = reduceSolveState(prev, { type: 'solveOk', payload, elapsedMs: 9 });
    expect(next).toEqual({ status: 'ok', payload, elapsedMs: 9 });
  });
});
