/// <reference lib="webworker" />

import type { ApiEnvelope, LangTag, SolvePayload } from './types';

interface EndWebModule {
  ccall(
    ident: string,
    returnType: 'number' | 'void',
    argTypes: Array<'string' | 'number'>,
    args: unknown[]
  ): number | undefined;
  UTF8ToString(ptr: number): string;
}

function callJsonApi<T>(module: EndWebModule, fnName: string, stringArgs: string[]): T {
  const ptr = module.ccall(
    fnName,
    'number',
    stringArgs.map(() => 'string'),
    stringArgs
  );

  if (!ptr || ptr <= 0) {
    throw new Error(`WASM function ${fnName} returned null pointer`);
  }

  try {
    const raw = module.UTF8ToString(ptr);
    const envelope = JSON.parse(raw) as ApiEnvelope<T>;

    if (envelope.status === 'err') {
      throw new Error(envelope.error.message);
    }

    return envelope.data;
  } finally {
    module.ccall('end_web_free_c_string', 'void', ['number'], [ptr]);
  }
}

interface EndWorkerGlobalScope extends DedicatedWorkerGlobalScope {
  createEndWebModule?: (opts?: Record<string, unknown>) => Promise<EndWebModule>;
}

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

let scriptLoaded = false;
let modulePromise: Promise<EndWebModule> | null = null;

function loadWasmScriptOnce(): void {
  if (scriptLoaded) {
    return;
  }

  importScripts('/wasm/end_web.js');
  scriptLoaded = true;
}

async function getModule(): Promise<EndWebModule> {
  if (modulePromise) {
    return modulePromise;
  }

  modulePromise = (async () => {
    loadWasmScriptOnce();

    const scope = self as EndWorkerGlobalScope;
    const factory = scope.createEndWebModule;
    if (!factory) {
      throw new Error(
        'createEndWebModule not found. Run `npm run build:wasm` in web/ first.'
      );
    }

    return factory({
      noInitialRun: true,
      locateFile: (path: string) => `/wasm/${path}`,
      printErr: (...args: unknown[]) => {
        console.error('[end-web wasm worker]', ...args);
      }
    });
  })();

  return modulePromise;
}

async function solveScenario(lang: LangTag, aicToml: string): Promise<SolvePayload> {
  const module = await getModule();
  return callJsonApi<SolvePayload>(module, 'end_web_solve_from_aic_toml', [lang, aicToml]);
}

const scope = self as EndWorkerGlobalScope;

scope.onmessage = (event: MessageEvent<SolveRequest>): void => {
  void (async () => {
    const request = event.data;
    if (request.kind !== 'solve') {
      return;
    }

    try {
      const payload = await solveScenario(request.lang, request.aicToml);
      const response: SolveOk = {
        id: request.id,
        kind: 'ok',
        payload
      };
      scope.postMessage(response);
    } catch (error) {
      const response: SolveErr = {
        id: request.id,
        kind: 'err',
        message: error instanceof Error ? error.message : String(error)
      };
      scope.postMessage(response);
    }
  })();
};
