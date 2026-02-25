export type TooltipValue = string | null | undefined;

type TooltipActionReturn = {
  update?: (value: TooltipValue) => void;
  destroy?: () => void;
};

const TOOLTIP_OFFSET_PX = 10;
const VIEWPORT_PADDING_PX = 8;

const TOOLTIP_STYLE_ELEMENT_ID = "end-tooltip-styles";

const TOOLTIP_CSS = `
/* Custom tooltip (replaces native \`title\`) */
.end-tooltip {
  position: fixed;
  z-index: 10000;
  max-width: min(360px, calc(100vw - 16px));
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--panel-strong);
  color: var(--ink);
  box-shadow: var(--shadow-popover);
  font-size: 12px;
  line-height: 1.35;
  pointer-events: none;
}

.end-tooltip-content {
  white-space: pre-wrap;
}
`;

function ensureTooltipStyles(): void {
  if (document.getElementById(TOOLTIP_STYLE_ELEMENT_ID)) {
    return;
  }

  const styleEl = document.createElement("style");
  styleEl.id = TOOLTIP_STYLE_ELEMENT_ID;
  styleEl.textContent = TOOLTIP_CSS;
  document.head.append(styleEl);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function resolveText(value: TooltipValue): string {
  const text = (value ?? "").trim();
  return text;
}

function positionTooltip(node: HTMLElement, tooltipEl: HTMLElement): void {
  const rect = node.getBoundingClientRect();
  const tipRect = tooltipEl.getBoundingClientRect();

  const viewportWidth = document.documentElement.clientWidth;
  const viewportHeight = document.documentElement.clientHeight;

  const idealLeft = rect.left + rect.width / 2 - tipRect.width / 2;
  const left = clamp(
    idealLeft,
    VIEWPORT_PADDING_PX,
    viewportWidth - VIEWPORT_PADDING_PX - tipRect.width,
  );

  const idealTop = rect.bottom + TOOLTIP_OFFSET_PX;
  const top = clamp(
    idealTop,
    VIEWPORT_PADDING_PX,
    viewportHeight - VIEWPORT_PADDING_PX - tipRect.height,
  );

  tooltipEl.style.left = `${left}px`;
  tooltipEl.style.top = `${top}px`;
}

function createTooltipElement(text: string): HTMLElement {
  const tooltipEl = document.createElement("div");
  tooltipEl.className = "end-tooltip";
  tooltipEl.setAttribute("role", "tooltip");

  const contentEl = document.createElement("div");
  contentEl.className = "end-tooltip-content";
  contentEl.textContent = text;

  tooltipEl.append(contentEl);
  return tooltipEl;
}

export function tooltip(node: HTMLElement, value: TooltipValue): TooltipActionReturn {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return {};
  }

  ensureTooltipStyles();

  let currentValue: TooltipValue = value;
  let tooltipEl: HTMLElement | null = null;
  const originalTitle = node.getAttribute("title");

  const removeNativeTitle = () => {
    if (node.hasAttribute("title")) {
      node.removeAttribute("title");
    }
  };

  const restoreNativeTitle = () => {
    if (originalTitle !== null && originalTitle.trim().length > 0) {
      node.setAttribute("title", originalTitle);
    }
  };

  const show = () => {
    const text = resolveText(currentValue);
    if (text.length === 0) {
      return;
    }

    removeNativeTitle();

    if (tooltipEl) {
      const content = tooltipEl.querySelector(".end-tooltip-content");
      if (content) {
        content.textContent = text;
      }
      positionTooltip(node, tooltipEl);
      return;
    }

    tooltipEl = createTooltipElement(text);
    document.body.append(tooltipEl);
    positionTooltip(node, tooltipEl);
  };

  const hide = () => {
    if (tooltipEl) {
      tooltipEl.remove();
      tooltipEl = null;
    }
  };

  const onPointerEnter = () => {
    show();
  };

  const onPointerLeave = () => {
    hide();
    restoreNativeTitle();
  };

  const onFocusIn = () => {
    show();
  };

  const onFocusOut = () => {
    hide();
    restoreNativeTitle();
  };

  const onScrollOrResize = () => {
    if (!tooltipEl) {
      return;
    }
    positionTooltip(node, tooltipEl);
  };

  node.addEventListener("mouseenter", onPointerEnter);
  node.addEventListener("mouseleave", onPointerLeave);
  node.addEventListener("focusin", onFocusIn);
  node.addEventListener("focusout", onFocusOut);

  window.addEventListener("scroll", onScrollOrResize, true);
  window.addEventListener("resize", onScrollOrResize);

  // Ensure we don't show native tooltip while our action is active.
  removeNativeTitle();

  return {
    update(nextValue: TooltipValue) {
      currentValue = nextValue;
      if (tooltipEl) {
        const nextText = resolveText(currentValue);
        if (nextText.length === 0) {
          hide();
        } else {
          const content = tooltipEl.querySelector(".end-tooltip-content");
          if (content) {
            content.textContent = nextText;
          }
          positionTooltip(node, tooltipEl);
        }
      }
    },
    destroy() {
      hide();
      restoreNativeTitle();
      node.removeEventListener("mouseenter", onPointerEnter);
      node.removeEventListener("mouseleave", onPointerLeave);
      node.removeEventListener("focusin", onFocusIn);
      node.removeEventListener("focusout", onFocusOut);
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    },
  };
}
