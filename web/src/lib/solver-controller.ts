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
  onSolvingChange: (next: boolean) => void;
  onErrorMessage: (next: string) => void;
  onSolved: (payload: SolvePayload, trigger: SolveTrigger) => void;
}

export interface SolverController {
  scheduleAutoSolve: () => void;
  runSolve: (trigger?: SolveTrigger) => Promise<void>;
  dispose: () => void;
  resetSolvedFingerprint: () => void;
}

export function createSolverController(options: CreateSolverControllerOptions): SolverController {
  let autoSolveTimer: number | null = null;
  let autoSolveDirty = false;
  let solveSequence = 0;
  let latestSolveSequence = 0;
  let lastSolvedFingerprint = '';
  let isSolving = false;
  let emittedSolving = false;

  function emitSolvingState(): void {
    const next = isSolving || autoSolveTimer !== null;
    if (next === emittedSolving) {
      return;
    }

    emittedSolving = next;
    options.onSolvingChange(next);
  }

  function clearAutoSolveTimer(): void {
    if (autoSolveTimer === null) {
      return;
    }
    window.clearTimeout(autoSolveTimer);
    autoSolveTimer = null;
    emitSolvingState();
  }

  function scheduleAutoSolve(): void {
    const snapshot = options.getSnapshot();
    if (snapshot.isBootstrapping) {
      return;
    }

    autoSolveDirty = true;
    clearAutoSolveTimer();
    autoSolveTimer = window.setTimeout(() => {
      autoSolveTimer = null;
      emitSolvingState();
      void runSolve('auto');
    }, options.debounceMs);
    emitSolvingState();
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
      options.onErrorMessage(error instanceof Error ? error.message : String(error));
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
    emitSolvingState();
    options.onErrorMessage('');

    try {
      const solved = await options.solve(snapshot.lang, toml);
      if (sequence !== latestSolveSequence) {
        return;
      }

      options.onSolved(solved, trigger);
      lastSolvedFingerprint = fingerprint;
    } catch (error) {
      if (sequence !== latestSolveSequence) {
        return;
      }
      options.onErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      if (sequence === latestSolveSequence) {
        isSolving = false;
        emitSolvingState();
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
