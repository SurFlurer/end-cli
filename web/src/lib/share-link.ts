function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...chunk);
  }

  const base64 = btoa(binary);
  return base64.replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function base64UrlToBytes(value: string): Uint8Array {
  const normalized = value.replaceAll("-", "+").replaceAll("_", "/");
  const pad = normalized.length % 4 === 0 ? "" : "=".repeat(4 - (normalized.length % 4));
  const base64 = normalized + pad;
  const binary = atob(base64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    out[i] = binary.charCodeAt(i);
  }
  return out;
}

async function compressDeflate(bytes: Uint8Array): Promise<Uint8Array> {
  const view = Uint8Array.from(bytes);
  const stream = new Blob([view])
    .stream()
    .pipeThrough(new CompressionStream("deflate"));
  const buffer = await new Response(stream).arrayBuffer();
  return new Uint8Array(buffer);
}

async function decompressDeflate(bytes: Uint8Array): Promise<Uint8Array> {
  const view = Uint8Array.from(bytes);
  const stream = new Blob([view])
    .stream()
    .pipeThrough(new DecompressionStream("deflate"));
  const buffer = await new Response(stream).arrayBuffer();
  return new Uint8Array(buffer);
}

export async function encodeTomlToShareParam(toml: string): Promise<string> {
  const raw = new TextEncoder().encode(toml);
  const compressed = await compressDeflate(raw);
  return bytesToBase64Url(compressed);
}

export async function decodeTomlFromShareParam(param: string): Promise<string> {
  const bytes = base64UrlToBytes(param);
  const decompressed = await decompressDeflate(bytes);
  return new TextDecoder().decode(decompressed);
}
