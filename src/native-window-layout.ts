const MINIMUM_LAYOUT_GUTTER = 24;

type NativeWindowMinimumSize = {
  width: number;
  height: number;
};

type NativeWindowLogicalSize = NativeWindowMinimumSize & {
  height: number;
};

type NativeWindowMinimumApplier = (size: NativeWindowMinimumSize) => Promise<void> | void;

type NativeWindowGeometry = {
  setMinimumSize: (size: NativeWindowMinimumSize) => Promise<void> | void;
  getCurrentLogicalSize?: () => Promise<NativeWindowLogicalSize> | NativeWindowLogicalSize;
  setCurrentLogicalSize?: (size: NativeWindowLogicalSize) => Promise<void> | void;
};

function numericCssPixels(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function inlineChrome(style: CSSStyleDeclaration): number {
  return (
    numericCssPixels(style.paddingLeft) +
    numericCssPixels(style.paddingRight) +
    numericCssPixels(style.borderLeftWidth) +
    numericCssPixels(style.borderRightWidth)
  );
}

function blockChrome(style: CSSStyleDeclaration): number {
  return (
    numericCssPixels(style.paddingTop) +
    numericCssPixels(style.paddingBottom) +
    numericCssPixels(style.borderTopWidth) +
    numericCssPixels(style.borderBottomWidth)
  );
}

function measureRectWidth(element: Element | null): number {
  return element?.getBoundingClientRect().width ?? 0;
}

function measureRectHeight(element: Element | null): number {
  return element?.getBoundingClientRect().height ?? 0;
}

function buildReadmeWidthProbe(root: HTMLElement): HTMLElement {
  const readmeInput = root.querySelector<HTMLElement>(".task-readme-control-input");
  const ownerWindow = root.ownerDocument.defaultView;
  const probe = root.ownerDocument.createElement("span");
  probe.className = "window-minimum-readme-probe";
  probe.setAttribute("aria-hidden", "true");
  probe.tabIndex = -1;
  probe.textContent = "0".repeat(80);
  const readmeStyle = readmeInput
    ? ownerWindow?.getComputedStyle(readmeInput)
    : null;
  if (readmeStyle) {
    probe.style.font = readmeStyle.font;
  }
  return probe;
}

function measureInspectorChrome(root: HTMLElement): number {
  const probe = root.ownerDocument.createElement("section");
  probe.className = "inspector-surface task-inspector window-minimum-inspector-probe";
  root.append(probe);
  try {
    const style = root.ownerDocument.defaultView?.getComputedStyle(probe);
    return style ? inlineChrome(style) : 0;
  } finally {
    probe.remove();
  }
}

function measureReadmeWidth(root: HTMLElement): number {
  const ownerWindow = root.ownerDocument.defaultView;
  const readmeInput = root.querySelector<HTMLElement>(".task-readme-control-input");
  const readmeStyle = readmeInput ? ownerWindow?.getComputedStyle(readmeInput) : null;
  const probe = buildReadmeWidthProbe(root);
  root.append(probe);
  try {
    return measureRectWidth(probe) + (readmeStyle ? inlineChrome(readmeStyle) : 0);
  } finally {
    probe.remove();
  }
}

function measureDocumentMinimumWidth(root: HTMLElement): number {
  const { body, documentElement } = root.ownerDocument;
  return Math.max(body?.scrollWidth ?? 0, documentElement?.scrollWidth ?? 0);
}

export function measureRenderedMinimumWindowSize(root: HTMLElement): NativeWindowMinimumSize {
  const ownerWindow = root.ownerDocument.defaultView;
  const sidebar = root.querySelector<HTMLElement>("[data-testid='shell-sidebar-host']");
  const workArea = root.querySelector<HTMLElement>("[data-testid='shell-main-host']");
  const timeline = root.querySelector<HTMLElement>("[data-testid='timeline-view']");
  const workAreaStyle = workArea && ownerWindow?.getComputedStyle(workArea);
  const sidebarWidth = measureRectWidth(sidebar);
  const workAreaInlinePadding = workAreaStyle ? inlineChrome(workAreaStyle) : 0;
  const workAreaBlockPadding = workAreaStyle ? blockChrome(workAreaStyle) : 0;
  const inspectorInlineChrome = measureInspectorChrome(root);
  const readmeWidth = measureReadmeWidth(root);
  const timelineHeight = measureRectHeight(timeline);

  const minimumReadableWidth =
    sidebarWidth +
    workAreaInlinePadding +
    inspectorInlineChrome +
    readmeWidth +
    MINIMUM_LAYOUT_GUTTER;

  return {
    height: Math.ceil(timelineHeight + workAreaBlockPadding + MINIMUM_LAYOUT_GUTTER),
    width: Math.ceil(Math.max(minimumReadableWidth, measureDocumentMinimumWidth(root))),
  };
}

export async function enforceNativeWindowMinimumSize(
  size: NativeWindowMinimumSize,
  geometry: NativeWindowGeometry,
): Promise<void> {
  await geometry.setMinimumSize(size);
  if (!geometry.getCurrentLogicalSize || !geometry.setCurrentLogicalSize) {
    return;
  }

  const currentSize = await geometry.getCurrentLogicalSize();
  const correctedSize = {
    width: Math.max(currentSize.width, size.width),
    height: Math.max(currentSize.height, size.height),
  };
  if (correctedSize.width !== currentSize.width || correctedSize.height !== currentSize.height) {
    await geometry.setCurrentLogicalSize({
      width: correctedSize.width,
      height: correctedSize.height,
    });
  }
}

export async function applyTauriNativeWindowMinimumSize(
  size: NativeWindowMinimumSize,
): Promise<void> {
  try {
    const { LogicalSize, getCurrentWindow } = await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    await enforceNativeWindowMinimumSize(size, {
      setMinimumSize: (nextSize) =>
        appWindow.setSizeConstraints({
          minWidth: nextSize.width,
          minHeight: nextSize.height,
        }),
      getCurrentLogicalSize: async () => {
        const scaleFactor = await appWindow.scaleFactor();
        const currentSize = (await appWindow.innerSize()).toLogical(scaleFactor);
        return { width: currentSize.width, height: currentSize.height };
      },
      setCurrentLogicalSize: (nextSize) =>
        appWindow.setSize(new LogicalSize(nextSize.width, nextSize.height)),
    });
  } catch {
    // Browser-only previews and unit tests do not expose the Tauri window API.
  }
}

export async function syncNativeWindowMinimumSize(
  root: HTMLElement,
  applyMinimumSize: NativeWindowMinimumApplier = applyTauriNativeWindowMinimumSize,
): Promise<NativeWindowMinimumSize> {
  const size = measureRenderedMinimumWindowSize(root);
  await applyMinimumSize(size);
  return size;
}
