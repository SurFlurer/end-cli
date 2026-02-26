import { elementToSVG } from "dom-to-svg";

import appCss from "../app.css?inline";
import xyflowCss from "@xyflow/svelte/dist/style.css?inline";

export type FlowExportSize = {
  width: number;
  height: number;
};

const FLOW_MAP_ID = "logistics-flow-map";

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

function findFlowMapElement(): HTMLElement {
  const element = document.getElementById(FLOW_MAP_ID);
  if (!element) {
    throw new Error(`Could not find flow map element: #${FLOW_MAP_ID}`);
  }
  return element;
}

function createOffscreenIframe(size: FlowExportSize): HTMLIFrameElement {
  const iframe = document.createElement("iframe");
  iframe.style.width = `${size.width}px`;
  iframe.style.height = `${size.height}px`;
  iframe.style.position = "fixed";
  iframe.style.left = "-10000px";
  iframe.style.top = "-10000px";
  iframe.style.opacity = "0";
  iframe.style.pointerEvents = "none";
  iframe.style.border = "0";
  return iframe;
}

function setupIframeDocument(
  iframeDocument: Document,
  size: FlowExportSize,
): HTMLElement {
  const docEl = iframeDocument.documentElement;
  const body = iframeDocument.body;

  docEl.style.width = `${size.width}px`;
  docEl.style.height = `${size.height}px`;
  body.style.margin = "0";
  body.style.width = `${size.width}px`;
  body.style.height = `${size.height}px`;
  body.style.overflow = "hidden";

  const styleEl = iframeDocument.createElement("style");
  styleEl.textContent = `${appCss}\n${xyflowCss}`;
  (iframeDocument.head ?? iframeDocument.documentElement).append(styleEl);

  const wrapper = iframeDocument.createElement("div");
  wrapper.style.width = `${size.width}px`;
  wrapper.style.height = `${size.height}px`;
  wrapper.style.overflow = "hidden";
  body.append(wrapper);
  return wrapper;
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

  const source = findFlowMapElement();
  const iframe = createOffscreenIframe(size);
  document.body.append(iframe);

  try {
    const iframeDocument = iframe.contentDocument;
    if (!iframeDocument) {
      throw new Error("Could not get iframe document");
    }

    const wrapper = setupIframeDocument(iframeDocument, size);
    const clone = source.cloneNode(true) as HTMLElement;

    clone.style.width = "100%";
    clone.style.height = "100%";
    wrapper.append(clone);

    const svgDocument = elementToSVG(iframeDocument.documentElement);
    return serializeSvgDocument(svgDocument);
  } finally {
    iframe.remove();
  }
}

export async function exportCurrentFlowToPngBlob(
  rawSize: FlowExportSize,
): Promise<Blob> {
  const svg = await exportCurrentFlowToSvgString(rawSize);
  const size = clampExportSize(rawSize);
  return svgStringToPngBlob(svg, size);
}
