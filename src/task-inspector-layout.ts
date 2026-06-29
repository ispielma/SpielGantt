type TaskInspectorLayoutMetrics = {
  columnGap: number;
  dependenciesMinWidth: number;
  inspectorWidth: number;
  readmeMinWidth: number;
};

export type TaskInspectorLayout = "columns" | "stacked";

function numericCssPixels(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function chooseTaskInspectorLayout(
  metrics: TaskInspectorLayoutMetrics,
): TaskInspectorLayout {
  const requiredColumnsWidth =
    metrics.readmeMinWidth + metrics.dependenciesMinWidth + metrics.columnGap;
  return metrics.inspectorWidth >= requiredColumnsWidth ? "columns" : "stacked";
}

function measureCssCustomLength(host: HTMLElement, propertyName: string): number {
  const probe = host.ownerDocument.createElement("div");
  probe.style.position = "absolute";
  probe.style.visibility = "hidden";
  probe.style.pointerEvents = "none";
  probe.style.width = `var(${propertyName})`;
  host.append(probe);
  try {
    return probe.getBoundingClientRect().width;
  } finally {
    probe.remove();
  }
}

export function measureTaskInspectorLayout(surface: HTMLElement): TaskInspectorLayout {
  const ownerWindow = surface.ownerDocument.defaultView;
  const layout = surface.querySelector<HTMLElement>("[data-testid='task-inspector-layout']");
  const readme = surface.querySelector<HTMLElement>("[data-testid='task-readme-section']");
  if (!ownerWindow || !layout || !readme) {
    return "stacked";
  }

  const layoutStyle = ownerWindow.getComputedStyle(layout);
  const readmeStyle = ownerWindow.getComputedStyle(readme);
  return chooseTaskInspectorLayout({
    columnGap: numericCssPixels(layoutStyle.columnGap),
    dependenciesMinWidth: measureCssCustomLength(surface, "--task-dependencies-panel-min-width"),
    inspectorWidth: layout.getBoundingClientRect().width,
    readmeMinWidth: numericCssPixels(readmeStyle.minWidth),
  });
}
