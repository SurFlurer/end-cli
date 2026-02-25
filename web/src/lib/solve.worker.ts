/* global importScripts */
/// <reference lib="webworker" />

import type { ApiEnvelope, BootstrapPayload, LangTag, SolvePayload } from './types';

interface EndWebModule {
  ccall(
    ident: string,
    returnType: 'number' | 'void',
    argTypes: ('string' | 'number')[],
    args: unknown[]
  ): number | undefined;
  HEAPU8: Uint8Array;
  HEAPU32: Uint32Array;
}

// Slice struct layout (32-bit WASM): ptr (4 bytes) + len (4 bytes) + cap (4 bytes)
const SLICE_SIZE = 12;

function normalizeBasePath(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed === '') {
    return '/';
  }

  const withLeadingSlash = trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
  return withLeadingSlash.endsWith('/') ? withLeadingSlash : `${withLeadingSlash}/`;
}

let wasmBase = '/wasm/';

function createSlice(module: EndWebModule, str: string): number | null {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(str);
  const len = bytes.length;
  
  const ptr = module.ccall('malloc', 'number', ['number'], [len]) as number;
  if (ptr === 0) {
    return null;
  }
  
  module.HEAPU8.set(bytes, ptr);
  
  const slicePtr = module.ccall('malloc', 'number', ['number'], [SLICE_SIZE]) as number;
  if (slicePtr === 0) {
    module.ccall('free', 'void', ['number'], [ptr]);
    return null;
  }
  
  module.HEAPU32[slicePtr >> 2] = ptr;
  module.HEAPU32[(slicePtr >> 2) + 1] = len;
  module.HEAPU32[(slicePtr >> 2) + 2] = len;
  
  return slicePtr;
}

function freeSlice(module: EndWebModule, slicePtr: number): void {
  if (slicePtr === 0) return;
  
  const strPtr = module.HEAPU32[slicePtr >> 2];
  if (strPtr !== 0) {
    module.ccall('free', 'void', ['number'], [strPtr]);
  }
  
  module.ccall('free', 'void', ['number'], [slicePtr]);
}

function callJsonApi<T>(module: EndWebModule, fnName: string, stringArgs: string[]): T {
  const inputSlices: number[] = [];
  for (const arg of stringArgs) {
    const slicePtr = createSlice(module, arg);
    if (slicePtr === null) {
      for (const ptr of inputSlices) {
        freeSlice(module, ptr);
      }
      throw new Error(`Failed to create slice for argument`);
    }
    inputSlices.push(slicePtr);
  }
  
  try {
    const resultSlicePtr = module.ccall(
      fnName,
      'number',
      inputSlices.map(() => 'number'),
      inputSlices
    ) as number;
    
    if (resultSlicePtr === 0) {
      throw new Error(`WASM function ${fnName} returned null`);
    }
    
    try {
      const strPtr = module.HEAPU32[resultSlicePtr >> 2];
      const strLen = module.HEAPU32[(resultSlicePtr >> 2) + 1];
      
      if (strPtr === 0) {
        throw new Error(`WASM function ${fnName} returned empty slice`);
      }
      
      const raw = new TextDecoder().decode(module.HEAPU8.subarray(strPtr, strPtr + strLen));
      const envelope = JSON.parse(raw) as ApiEnvelope<T>;
      
      if (envelope.status === 'err') {
        const sourceText = envelope.error.source?.trim();
        const message = sourceText
          ? `${envelope.error.message}: ${sourceText}`
          : envelope.error.message;
        throw new Error(message);
      }
      
      return envelope.data;
    } finally {
      module.ccall('end_web_free_slice', 'void', ['number'], [resultSlicePtr]);
    }
  } finally {
    for (const ptr of inputSlices) {
      freeSlice(module, ptr);
    }
  }
}

type EndWorkerGlobalScope = typeof self & {
  createEndWebModule?: (opts?: Record<string, unknown>) => Promise<EndWebModule>;
};

interface SolveRequest {
  id: number;
  kind: 'solve';
  lang: LangTag;
  aicToml: string;
}

interface BootstrapRequest {
  id: number;
  kind: 'bootstrap';
  lang: LangTag;
}

interface WarmupRequest {
  id: number;
  kind: 'warmup';
}

interface WorkerInitRequest {
  kind: 'init';
  wasmBase: string;
}

interface BootstrapOk {
  id: number;
  kind: 'bootstrap_ok';
  payload: BootstrapPayload;
}

interface BootstrapErr {
  id: number;
  kind: 'bootstrap_err';
  message: string;
}

interface WarmupOk {
  id: number;
  kind: 'warmup_ok';
}

interface WarmupErr {
  id: number;
  kind: 'warmup_err';
  message: string;
}

interface SolveOk {
  id: number;
  kind: 'solve_ok';
  payload: SolvePayload;
}

interface SolveErr {
  id: number;
  kind: 'solve_err';
  message: string;
}

type WorkerRequest = WorkerInitRequest | BootstrapRequest | WarmupRequest | SolveRequest;

let scriptLoaded = false;
let modulePromise: Promise<EndWebModule> | null = null;

function loadWasmScriptOnce(): void {
  if (scriptLoaded) {
    return;
  }

  importScripts(`${wasmBase}end_web.js`);
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
        'createEndWebModule not found. Run `npm run dev` to auto-build wasm, or run `npm run build:wasm` manually.'
      );
    }

    return factory({
      noInitialRun: true,
      locateFile: (path: string) => `${wasmBase}${path}`,
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

async function loadBootstrap(lang: LangTag): Promise<BootstrapPayload> {
  const module = await getModule();
  return callJsonApi<BootstrapPayload>(module, 'end_web_bootstrap', [lang]);
}

const scope = self as EndWorkerGlobalScope;

scope.onmessage = (event: MessageEvent<WorkerRequest>): void => {
  void (async () => {
    const request = event.data;
    if (request.kind === 'init') {
      wasmBase = normalizeBasePath(request.wasmBase);
      return;
    }

    if (request.kind === 'bootstrap') {
      try {
        const payload = await loadBootstrap(request.lang);
        const response: BootstrapOk = {
          id: request.id,
          kind: 'bootstrap_ok',
          payload
        };
        scope.postMessage(response);
      } catch (error) {
        const response: BootstrapErr = {
          id: request.id,
          kind: 'bootstrap_err',
          message: error instanceof Error ? error.message : String(error)
        };
        scope.postMessage(response);
      }
      return;
    }

    if (request.kind === 'warmup') {
      try {
        await getModule();
        const response: WarmupOk = {
          id: request.id,
          kind: 'warmup_ok'
        };
        scope.postMessage(response);
      } catch (error) {
        const response: WarmupErr = {
          id: request.id,
          kind: 'warmup_err',
          message: error instanceof Error ? error.message : String(error)
        };
        scope.postMessage(response);
      }
      return;
    }

    if (request.kind !== 'solve') {
      return;
    }

    try {
      const payload = await solveScenario(request.lang, request.aicToml);
      const response: SolveOk = {
        id: request.id,
        kind: 'solve_ok',
        payload
      };
      scope.postMessage(response);
    } catch (error) {
      const response: SolveErr = {
        id: request.id,
        kind: 'solve_err',
        message: error instanceof Error ? error.message : String(error)
      };
      scope.postMessage(response);
    }
  })();
};
