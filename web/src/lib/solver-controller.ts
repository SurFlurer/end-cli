import { reduceSolveState } from './solve-state';
import type { SolveState } from './solve-state';
import type { AicDraft, LangTag, SolvePayload } from './types';

export type SolveTrigger = 'auto' | 'manual';

export interface SolveSnapshot {
  draft: AicDraft;
  lang: LangTag;
  isBootstrapping: boolean;
}

interface CreateSolverControllerOptions {
  debounceMs: number;
  getSnapshot: () => SolveSnapshot;
  toToml: (draft: AicDraft) => string;
  solve: (lang: LangTag, toml: string) => Promise<SolvePayload>;
  onStateChange: (next: SolveState) => void;
  nowMs?: () => number;
}

export interface SolverController {
  scheduleAutoSolve: () => void;
  runSolve: (trigger?: SolveTrigger) => Promise<void>;
  dispose: () => void;
  resetSolvedFingerprint: () => void;
}

export function createSolverController(options: CreateSolverControllerOptions): SolverController {
  const nowMs = options.nowMs ?? (() => performance.now());

  let autoSolveTimer: number | null = null;
  let autoSolveDirty = false;
  let solveSequence = 0;
  let latestSolveSequence = 0;
  let lastSolvedFingerprint = '';
  let isSolving = false;

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
    window.clearTimeout(autoSolveTimer);
    autoSolveTimer = null;
    applyEvent({ type: 'debounceCleared' });
  }

  function scheduleAutoSolve(): void {
    const snapshot = options.getSnapshot();
    if (snapshot.isBootstrapping) {
      return;
    }

    if (isSolving) {
      autoSolveDirty = true;
      return;
    }

    autoSolveDirty = true;
    clearAutoSolveTimer();
    autoSolveTimer = window.setTimeout(() => {
      autoSolveTimer = null;
      applyEvent({ type: 'debounceCleared' });
      void runSolve('auto');
    }, options.debounceMs);
    applyEvent({ type: 'debounceScheduled' });
  }

  async function runSolve(trigger: SolveTrigger = 'manual'): Promise<void> {
    const snapshot = options.getSnapshot();
    if (snapshot.isBootstrapping) {
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

  return {
    scheduleAutoSolve,
    runSolve,
    dispose,
    resetSolvedFingerprint
  };
}
