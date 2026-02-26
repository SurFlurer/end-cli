import { elementToSVG } from "dom-to-svg";
import { mount, unmount } from "svelte";
import { get } from "svelte/store";
import type { Node } from "@xyflow/svelte";
import FlowExportRenderer from "./FlowExportRenderer.svelte";
import { currentFlowSnapshot, type FlowSnapshot } from "./flow-snapshot";

type ExportSize = {
  width: number;
  height: number;
};

type NodeBounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

function offscreenDiv(size: ExportSize): HTMLDivElement {
  // Keep it fully opaque.
  // dom-to-svg treats `opacity: 0` as invisible and skips it (and all descendants),
  // which breaks export unless debug mode forces opacity back to 1.

  const root = document.createElement("div");
  root.style.width = `${size.width}px`;
  root.style.height = `${size.height}px`;
  root.style.position = "fixed";
  root.style.left = "-10000px";
  root.style.top = "-10000px";
  root.style.pointerEvents = "none";
  root.style.overflow = "hidden";
  return root;
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function nodesBounds(nodes: Node[]): NodeBounds {
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of nodes) {
    const x = node.position.x;
    const y = node.position.y;

    const width = node.measured?.width ?? node.width ?? 220;
    const height = node.measured?.height ?? node.height ?? 44;

    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + width);
    maxY = Math.max(maxY, y + height);
  }

  return { minX, minY, maxX, maxY };
}

function exportSizeFromNodes(nodes: Node[]): ExportSize {
  const size = (max: number, min: number) => Math.ceil(Math.max(1, max - min));
  const bounds = nodesBounds(nodes);

  return {
    width: size(bounds.maxX, bounds.minX),
    height: size(bounds.maxY, bounds.minY),
  };
}

function fitViewportToNodes(
  nodes: Node[],
  size: ExportSize,
): { x: number; y: number; zoom: number } {
  const bounds = nodesBounds(nodes);

  const boundsWidth = Math.max(1, bounds.maxX - bounds.minX);
  const boundsHeight = Math.max(1, bounds.maxY - bounds.minY);

  // Match typical fitView defaults: ~10% padding.
  const padding = 0.1;
  const paddedMinX = bounds.minX - boundsWidth * padding;
  const paddedMinY = bounds.minY - boundsHeight * padding;
  const paddedWidth = boundsWidth * (1 + padding * 2);
  const paddedHeight = boundsHeight * (1 + padding * 2);

  const rawZoom = Math.min(size.width / paddedWidth, size.height / paddedHeight);
  const zoom = Math.max(0.05, Math.min(2, rawZoom));

  const x = (size.width - paddedWidth * zoom) / 2 - paddedMinX * zoom;
  const y = (size.height - paddedHeight * zoom) / 2 - paddedMinY * zoom;
  return { x, y, zoom };
}

function serializeSvgDocument(svgDocument: Document): string {
  const serialized = new XMLSerializer().serializeToString(svgDocument);
  // Ensure the string can be used as a standalone SVG payload.
  return serialized.startsWith("<?xml") ? serialized : `<?xml version="1.0" encoding="UTF-8"?>\n${serialized}`;
}

async function svgStringToPngBlob(
  svgString: string,
): Promise<Blob> {
  const svgUrl = URL.createObjectURL(new Blob([svgString], { type: "image/svg+xml" }));

  try {
    // Prefer HTMLImageElement decoding for SVG, since createImageBitmap(svgBlob)
    // is not consistently supported for SVG across browsers and may throw
    // "The source image could not be decoded".
    // `decode()` is supported in modern browsers
    const img = new Image();
    img.decoding = "async";
    img.src = svgUrl;
    await img.decode();

    const width = img.naturalWidth
    const height = img.naturalHeight

    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0, width, height);

    const png = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (blob) => {
          if (blob) {
            resolve(blob);
          } else {
            reject(new Error("Failed to render PNG"));
          }
        },
        "image/png",
        1,
      );
    });

    return png;
  } finally {
    URL.revokeObjectURL(svgUrl);
  }
}

export async function exportCurrentFlowToSvgString(snapshot: FlowSnapshot): Promise<string> {
  const nodes = snapshot.nodes;
  const edges = snapshot.edges;

  const size = exportSizeFromNodes(nodes);
  const viewport = fitViewportToNodes(nodes, size);

  const { promise: ready, resolve: markReady } = deferred<void>();

  const root = offscreenDiv(size);
  document.body.append(root);
  const exportApp = mount(FlowExportRenderer, {
    target: root,
    props: {
      nodes: nodes,
      edges: edges,
      viewport: viewport,
      width: size.width,
      height: size.height,
      onReady: markReady
    },
  });

  try {
    await ready;

    const svgDocument = elementToSVG(root);
    return serializeSvgDocument(svgDocument);
  } finally {
    unmount(exportApp);
    root.remove();
  }
}

export async function exportCurrentFlowToPngBlob(
): Promise<Blob> {
  const snapshot = get(currentFlowSnapshot);
  if (!snapshot) {
    throw new Error("Flow is not ready to export");
  }

  const svg = await exportCurrentFlowToSvgString(snapshot);
  return svgStringToPngBlob(svg);
}
