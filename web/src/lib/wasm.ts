import type { BootstrapPayload, LangTag, SolvePayload } from './types';
import { callJsonApi, type EndWebModule } from './wasm-core';

interface SolveRequest {
  id: number;
  kind: 'solve';
  lang: LangTag;
  aicToml: string;
}

interface SolveOk {
  id: number;
  kind: 'ok';
  payload: SolvePayload;
}

interface SolveErr {
  id: number;
  kind: 'err';
  message: string;
}

type SolveResponse = SolveOk | SolveErr;

declare global {
  interface Window {
    createEndWebModule?: (opts?: Record<string, unknown>) => Promise<EndWebModule>;
  }
}

let modulePromise: Promise<EndWebModule> | null = null;
let scriptPromise: Promise<void> | null = null;
let solveWorker: Worker | null = null;
let solveRequestId = 1;

const pendingSolve = new Map<
  number,
  {
    resolve: (value: SolvePayload) => void;
    reject: (reason?: unknown) => void;
  }
>();

class WorkerTransportError extends Error {}

function rejectAllPendingSolves(error: Error): void {
  for (const pending of pendingSolve.values()) {
    pending.reject(error);
  }
  pendingSolve.clear();
}

function getSolveWorker(): Worker {
  if (solveWorker) {
    return solveWorker;
  }

  solveWorker = new Worker(new URL('./solve.worker.ts', import.meta.url), {
    type: 'classic'
  });
  solveWorker.onmessage = (event: MessageEvent<SolveResponse>) => {
    const response = event.data;
    const pending = pendingSolve.get(response.id);
    if (!pending) {
      return;
    }

    pendingSolve.delete(response.id);
    if (response.kind === 'ok') {
      pending.resolve(response.payload);
      return;
    }

    pending.reject(new Error(response.message));
  };

  solveWorker.onerror = (event: ErrorEvent) => {
    solveWorker = null;
    rejectAllPendingSolves(new WorkerTransportError(event.message || 'solve worker crashed'));
  };

  return solveWorker;
}

function loadScriptOnce(src: string): Promise<void> {
  if (scriptPromise) {
    return scriptPromise;
  }

  scriptPromise = new Promise<void>((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>(
      `script[data-end-web-src="${src}"]`
    );
    if (existing) {
      resolve();
      return;
    }

    const script = document.createElement('script');
    script.src = src;
    script.async = true;
    script.dataset.endWebSrc = src;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error(`failed to load ${src}`));
    document.head.appendChild(script);
  });

  return scriptPromise;
}

async function getModule(): Promise<EndWebModule> {
  if (modulePromise) {
    return modulePromise;
  }

  modulePromise = (async () => {
    await loadScriptOnce('/wasm/end_web.js');

    const factory = window.createEndWebModule;
    if (!factory) {
      throw new Error(
        'createEndWebModule not found. Run `npm run build:wasm` in web/ first.'
      );
    }

    const module = await factory({
      noInitialRun: true,
      locateFile: (path: string) => `/wasm/${path}`,
      printErr: (...args: unknown[]) => {
        console.error('[end-web wasm]', ...args);
      }
    });

    return module;
  })();

  return modulePromise;
}

export async function loadBootstrap(lang: LangTag): Promise<BootstrapPayload> {
  const module = await getModule();
  return callJsonApi<BootstrapPayload>(module, 'end_web_bootstrap', [lang]);
}

export function solveScenario(lang: LangTag, aicToml: string): Promise<SolvePayload> {
  const worker = getSolveWorker();
  const request: SolveRequest = {
    id: solveRequestId,
    kind: 'solve',
    lang,
    aicToml
  };
  solveRequestId += 1;

  return new Promise<SolvePayload>((resolve, reject) => {
    pendingSolve.set(request.id, { resolve, reject });

    try {
      worker.postMessage(request);
    } catch (error) {
      solveWorker = null;
      pendingSolve.delete(request.id);
      reject(new WorkerTransportError(error instanceof Error ? error.message : String(error)));
    }
  });
}
