import type { ApiEnvelope } from './types';

export interface EndWebModule {
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

function createSlice(module: EndWebModule, str: string): number | null {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(str);
  const len = bytes.length;
  
  // Allocate string buffer
  const ptr = module.ccall('malloc', 'number', ['number'], [len]) as number;
  if (ptr === 0) {
    return null;
  }
  
  // Copy string data
  module.HEAPU8.set(bytes, ptr);
  
  // Allocate Slice struct
  const slicePtr = module.ccall('malloc', 'number', ['number'], [SLICE_SIZE]) as number;
  if (slicePtr === 0) {
    module.ccall('free', 'void', ['number'], [ptr]);
    return null;
  }
  
  // Write Slice fields
  module.HEAPU32[slicePtr >> 2] = ptr;
  module.HEAPU32[(slicePtr >> 2) + 1] = len;
  module.HEAPU32[(slicePtr >> 2) + 2] = len;
  
  return slicePtr;
}

function freeSlice(module: EndWebModule, slicePtr: number): void {
  if (slicePtr === 0) return;
  
  // Read ptr field and free the string buffer
  const strPtr = module.HEAPU32[slicePtr >> 2];
  if (strPtr !== 0) {
    module.ccall('free', 'void', ['number'], [strPtr]);
  }
  
  // Free the Slice struct itself
  module.ccall('free', 'void', ['number'], [slicePtr]);
}

export function callJsonApi<T>(module: EndWebModule, fnName: string, stringArgs: string[]): T {
  // Create input Slices
  const inputSlices: number[] = [];
  for (const arg of stringArgs) {
    const slicePtr = createSlice(module, arg);
    if (slicePtr === null) {
      // Cleanup already allocated slices
      for (const ptr of inputSlices) {
        freeSlice(module, ptr);
      }
      throw new Error(`Failed to create slice for argument`);
    }
    inputSlices.push(slicePtr);
  }
  
  try {
    // Call FFI function
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
      // Read result Slice
      const strPtr = module.HEAPU32[resultSlicePtr >> 2];
      const strLen = module.HEAPU32[(resultSlicePtr >> 2) + 1];
      
      if (strPtr === 0) {
        throw new Error(`WASM function ${fnName} returned empty slice`);
      }
      
      // Decode string
      const raw = new TextDecoder().decode(module.HEAPU8.subarray(strPtr, strPtr + strLen));
      const envelope = JSON.parse(raw) as ApiEnvelope<T>;
      
      if (envelope.status === 'err') {
        throw new Error(envelope.error.message);
      }
      
      return envelope.data;
    } finally {
      // Free result Slice using Rust's free function
      module.ccall('end_web_free_slice', 'void', ['number'], [resultSlicePtr]);
    }
  } finally {
    // Free input slices
    for (const ptr of inputSlices) {
      freeSlice(module, ptr);
    }
  }
}
