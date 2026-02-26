import { elementToSVG } from "dom-to-svg";
import { mount, unmount } from "svelte";
import { get } from "svelte/store";

import FlowExportRenderer from "../components/FlowExportRenderer.svelte";
import { currentFlowSnapshot } from "./flow-snapshot";

export type FlowExportSize = {
  width: number;
  height: number;
};

type NodeBounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

type FlowNodeLike = {
  position?: { x?: unknown; y?: unknown };
  positionAbsolute?: { x?: unknown; y?: unknown };
  measured?: { width?: unknown; height?: unknown };
  width?: unknown;
  height?: unknown;
  style?: unknown;
};

type ExportFlowDebugConfig = {
  enabled: boolean;
  showIframe: boolean;
  keepIframe: boolean;
};

function getExportFlowDebugConfig(): ExportFlowDebugConfig {
  // Debug helpers should be dev-only; exporting in production should stay clean.
  if (!import.meta.env.DEV) {
    return { enabled: false, showIframe: false, keepIframe: false };
  }

  // Enable via either:
  // - URL params: ?debugExportFlow=1&debugExportFlowShow=1&debugExportFlowKeep=1
  // - or localStorage: debugExportFlow=1, debugExportFlowShow=1, debugExportFlowKeep=1
  const params = new URLSearchParams(window.location.search);
  const hasParam = (key: string): boolean => params.has(key);
  const ls = window.localStorage;
  const hasLs = (key: string): boolean => ls.getItem(key) === "1";

  const enabled = hasParam("debugExportFlow") || hasLs("debugExportFlow");
  const showIframe = enabled && (hasParam("debugExportFlowShow") || hasLs("debugExportFlowShow"));
  const keepIframe = enabled && (hasParam("debugExportFlowKeep") || hasLs("debugExportFlowKeep"));
  return { enabled, showIframe, keepIframe };
}

function clampExportSize(size: FlowExportSize): FlowExportSize {
  const width = Math.max(240, Math.min(8192, Math.trunc(size.width)));
  const height = Math.max(240, Math.min(8192, Math.trunc(size.height)));
  return { width, height };
}

function ensureBrowser(): void {
  if (typeof window === "undefined" || typeof document === "undefined") {
    throw new Error("export is unavailable in this environment");
  }
}

function createOffscreenExportRoot(size: FlowExportSize): HTMLDivElement {
  const root = document.createElement("div");
  root.style.width = `${size.width}px`;
  root.style.height = `${size.height}px`;
  root.style.position = "fixed";
  root.style.left = "-10000px";
  root.style.top = "-10000px";
  root.style.opacity = "0";
  root.style.pointerEvents = "none";
  root.style.overflow = "hidden";
  root.style.background = "transparent";
  root.dataset.exportFlowRoot = "1";
  return root;
}

function applyDebugStylesToExportRoot(
  root: HTMLDivElement,
  size: FlowExportSize,
  debug: ExportFlowDebugConfig,
): void {
  if (!debug.enabled) {
    return;
  }

  root.dataset.exportFlowDebug = "1";
  root.style.opacity = "1";

  if (debug.showIframe) {
    // Keep the same query param naming, but now it shows the export root.
    root.style.left = "0";
    root.style.top = "0";
    root.style.zIndex = "2147483647";
    root.style.pointerEvents = "auto";
    root.style.border = "1px solid currentColor";
    root.style.background = "white";
    root.style.width = `${size.width}px`;
    root.style.height = `${size.height}px`;
  }

  (window as Window & { __exportFlowDebugRoot?: HTMLDivElement }).__exportFlowDebugRoot = root;
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
}

function parseInlineStylePx(style: unknown, property: string): number | null {
  if (typeof style !== "string") {
    return null;
  }
  // e.g. "width:12px;min-width:220px;"
  const needle = `${property}:`;
  const idx = style.indexOf(needle);
  if (idx < 0) {
    return null;
  }
  const after = style.slice(idx + needle.length);
  const match = after.match(/\s*([0-9]+(?:\.[0-9]+)?)px/i);
  if (!match) {
    return null;
  }
  const parsed = Number.parseFloat(match[1]);
  return Number.isFinite(parsed) ? parsed : null;
}

function computeNodesBounds(nodes: FlowNodeLike[]): NodeBounds | null {
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of nodes) {
    const posAbs = node.positionAbsolute;
    const pos = node.position ?? {};
    const xRaw = posAbs?.x ?? pos.x;
    const yRaw = posAbs?.y ?? pos.y;
    const x = typeof xRaw === "number" ? xRaw : Number.NaN;
    const y = typeof yRaw === "number" ? yRaw : Number.NaN;
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      continue;
    }

    const measured = node.measured;
    const widthRaw = measured?.width ?? node.width;
    const heightRaw = measured?.height ?? node.height;
    const style = node.style;

    const width =
      (typeof widthRaw === "number" && Number.isFinite(widthRaw) && widthRaw > 0
        ? widthRaw
        : null) ??
      parseInlineStylePx(style, "width") ??
      parseInlineStylePx(style, "min-width") ??
      220;

    const height =
      (typeof heightRaw === "number" && Number.isFinite(heightRaw) && heightRaw > 0
        ? heightRaw
        : null) ??
      parseInlineStylePx(style, "height") ??
      44;

    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + width);
    maxY = Math.max(maxY, y + height);
  }

  if (!Number.isFinite(minX) || !Number.isFinite(minY) || !Number.isFinite(maxX) || !Number.isFinite(maxY)) {
    return null;
  }
  return { minX, minY, maxX, maxY };
} 

function fitViewportToNodes(
  nodes: FlowNodeLike[],
  size: FlowExportSize,
): { x: number; y: number; zoom: number } | null {
  const bounds = computeNodesBounds(nodes);
  if (!bounds) {
    return null;
  }

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
  size: FlowExportSize,
): Promise<Blob> {
  const svgBlob = new Blob([svgString], { type: "image/svg+xml" });
  const svgUrl = URL.createObjectURL(svgBlob);

  try {
    const canvas = document.createElement("canvas");
    canvas.width = size.width;
    canvas.height = size.height;

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to create canvas context");
    }

    let drew = false;
    if ("createImageBitmap" in window) {
      try {
        const bitmap = await createImageBitmap(svgBlob);
        ctx.drawImage(bitmap, 0, 0, size.width, size.height);
        bitmap.close();
        drew = true;
      } catch {
        // Some browsers expose createImageBitmap but don't support SVG blobs.
      }
    }

    if (!drew) {
      const img = new Image();
      img.decoding = "async";
      img.src = svgUrl;
      await img.decode();
      ctx.drawImage(img, 0, 0, size.width, size.height);
    }

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

export async function exportCurrentFlowToSvgString(
  rawSize: FlowExportSize,
): Promise<string> {
  ensureBrowser();
  const size = clampExportSize(rawSize);
  const debug = getExportFlowDebugConfig();
  const snapshot = get(currentFlowSnapshot);

  if (!snapshot) {
    throw new Error("Flow is not ready to export");
  }

  const root = createOffscreenExportRoot(size);
  applyDebugStylesToExportRoot(root, size, debug);
  document.body.append(root);

  const exportApp = mount(FlowExportRenderer, {
    target: root,
    props: {
      nodes: snapshot.nodes,
      edges: snapshot.edges,
      viewport: fitViewportToNodes(snapshot.nodes as unknown as FlowNodeLike[], size) ?? snapshot.viewport,
      width: size.width,
      height: size.height,
    },
  });

  try {
    // Allow SvelteFlow to layout/measure.
    await nextAnimationFrame();
    await nextAnimationFrame();

    const svgDocument = elementToSVG(root);
    return serializeSvgDocument(svgDocument);
  } finally {
    if (!debug.keepIframe) {
      unmount(exportApp);
      root.remove();
    } else {
      (window as Window & { __exportFlowDebugApp?: unknown }).__exportFlowDebugApp = exportApp;
    }
  }
}

export async function exportCurrentFlowToPngBlob(
  rawSize: FlowExportSize,
): Promise<Blob> {
  const svg = await exportCurrentFlowToSvgString(rawSize);
  const size = clampExportSize(rawSize);
  return svgStringToPngBlob(svg, size);
}
