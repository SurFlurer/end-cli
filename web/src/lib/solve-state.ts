import type { SolvePayload } from './types';

export type SolveOkState = {
  status: 'ok';
  payload: SolvePayload;
  elapsedMs: number;
};

export type SolveEvent =
  | { type: 'debounceScheduled' }
  | { type: 'debounceCleared' }
  | { type: 'solveStarted'; startedAt: number }
  | { type: 'solveOk'; payload: SolveOkState['payload']; elapsedMs: number }
  | { type: 'solveErr'; message: string }
  | { type: 'clearError' };

export type SolveState =
  | { status: 'idle' }
  | { status: 'pending'; previousOk: SolveOkState | null } // debouncing, not yet started solving
  | { status: 'solving'; startedAt: number; previousOk: SolveOkState | null } // solve in progress
  | SolveOkState
  | { status: 'err'; message: string };

export function isSolveBusy(state: SolveState): boolean {
  return state.status === 'pending' || state.status === 'solving';
}

export function renderedOkState(state: SolveState): SolveOkState | null {
  if (state.status === 'ok') {
    return state;
  }

  if (state.status === 'pending' || state.status === 'solving') {
    return state.previousOk;
  }

  return null;
}

export function errorMessageOf(state: SolveState): string {
  return state.status === 'err' ? state.message : '';
}

export function reduceSolveState(prev: SolveState, event: SolveEvent): SolveState {
  if (event.type === 'debounceScheduled') {
    return {
      status: 'pending',
      previousOk: renderedOkState(prev)
    };
  }

  if (event.type === 'debounceCleared') {
    if (prev.status !== 'pending') {
      return prev;
    }

    return prev.previousOk ?? { status: 'idle' };
  }

  if (event.type === 'solveStarted') {
    return {
      status: 'solving',
      startedAt: event.startedAt,
      previousOk: renderedOkState(prev)
    };
  }

  if (event.type === 'solveOk') {
    return {
      status: 'ok',
      payload: event.payload,
      elapsedMs: event.elapsedMs
    };
  }

  if (event.type === 'clearError') {
    return prev.status === 'err' ? { status: 'idle' } : prev;
  }

  const message = event.message.trim();
  if (message.length === 0) {
    return prev.status === 'err' ? { status: 'idle' } : prev;
  }

  return {
    status: 'err',
    message
  };
}
