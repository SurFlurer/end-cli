import { reduceSolveState } from './solve-state';
import type { SolveState } from './solve-state';
import type { AicDraft, LangTag, SolvePayload } from './types';

export interface SolveSnapshot {
  draft: AicDraft;
  lang: LangTag;
  isBootstrapping: boolean;
}

type TimeoutHandle = ReturnType<typeof globalThis.setTimeout>;

export interface TimeoutApi {
  setTimeout: (handler: () => void, delayMs: number) => TimeoutHandle;
  clearTimeout: (timeout: TimeoutHandle) => void;
}

interface CreateSolverControllerOptions {
  debounceMs: number;
  toToml: (draft: AicDraft) => string;
  solve: (lang: LangTag, toml: string) => Promise<SolvePayload>;
  onStateChange: (next: SolveState) => void;
  timeoutApi: TimeoutApi;
  nowMs?: () => number;
}

export interface SolverController {
  updateSnapshot: (snapshot: SolveSnapshot) => void;
  runSolve: () => Promise<void>;
  dispose: () => void;
  resetSolvedFingerprint: () => void;
}

export function createSolverController(options: CreateSolverControllerOptions): SolverController {
  const nowMs = options.nowMs ?? (() => (typeof performance === 'undefined' ? Date.now() : performance.now()));

  let autoSolveTimer: TimeoutHandle | null = null;
  let autoSolveDirty = false;
  let solveSequence = 0;
  let latestSolveSequence = 0;
  let lastSolvedFingerprint = '';
  let isSolving = false;
  let latestSnapshot: SolveSnapshot | null = null;

  let state: SolveState = { status: 'idle' };
  options.onStateChange(state);

  function applyEvent(event: Parameters<typeof reduceSolveState>[1]): void {
    const next = reduceSolveState(state, event);
    if (next === state) {
      return;
    }
    state = next;
    options.onStateChange(state);
  }

  function clearAutoSolveTimer(): void {
    if (autoSolveTimer === null) {
      return;
    }
    options.timeoutApi.clearTimeout(autoSolveTimer);
    autoSolveTimer = null;
    applyEvent({ type: 'debounceCleared' });
  }

  function scheduleAutoSolve(): void {
    const snapshot = latestSnapshot;
    if (!snapshot || snapshot.isBootstrapping) {
      return;
    }

    if (isSolving) {
      autoSolveDirty = true;
      return;
    }

    autoSolveDirty = true;
    clearAutoSolveTimer();
    autoSolveTimer = options.timeoutApi.setTimeout(() => {
      autoSolveTimer = null;
      applyEvent({ type: 'debounceCleared' });
      void runSolve();
    }, options.debounceMs);
    applyEvent({ type: 'debounceScheduled' });
  }

  async function runSolve(): Promise<void> {
    const snapshot = latestSnapshot;
    if (!snapshot || snapshot.isBootstrapping) {
      return;
    }

    if (isSolving) {
      autoSolveDirty = true;
      return;
    }

    autoSolveDirty = false;

    let toml = '';
    try {
      toml = options.toToml(snapshot.draft);
    } catch (error) {
      applyEvent({
        type: 'solveErr',
        message: error instanceof Error ? error.message : String(error)
      });
      return;
    }

    const fingerprint = `${snapshot.lang}\n${toml}`;
    if (fingerprint === lastSolvedFingerprint) {
      return;
    }

    solveSequence += 1;
    const sequence = solveSequence;
    latestSolveSequence = sequence;

    isSolving = true;
    applyEvent({ type: 'clearError' });
    const startedAt = nowMs();
    applyEvent({ type: 'solveStarted', startedAt });

    try {
      const solved = await options.solve(snapshot.lang, toml);
      if (sequence !== latestSolveSequence) {
        return;
      }

      const elapsedMs = Math.max(0, Math.round(nowMs() - startedAt));
      applyEvent({ type: 'solveOk', payload: solved, elapsedMs });
      lastSolvedFingerprint = fingerprint;
    } catch (error) {
      if (sequence !== latestSolveSequence) {
        return;
      }
      applyEvent({
        type: 'solveErr',
        message: error instanceof Error ? error.message : String(error)
      });
    } finally {
      if (sequence === latestSolveSequence) {
        isSolving = false;
      }

      if (autoSolveDirty) {
        scheduleAutoSolve();
      }
    }
  }

  function dispose(): void {
    clearAutoSolveTimer();
  }

  function resetSolvedFingerprint(): void {
    lastSolvedFingerprint = '';
  }

  function updateSnapshot(snapshot: SolveSnapshot): void {
    latestSnapshot = snapshot;
    scheduleAutoSolve();
  }

  return {
    updateSnapshot,
    runSolve,
    dispose,
    resetSolvedFingerprint
  };
}
