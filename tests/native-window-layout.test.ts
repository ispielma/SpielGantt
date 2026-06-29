import assert from "node:assert/strict";
import test from "node:test";
import { Window } from "happy-dom";

import {
  enforceNativeWindowMinimumSize,
  measureRenderedMinimumWindowSize,
  syncNativeWindowMinimumSize,
} from "../src/native-window-layout.ts";

function rectWithSize(width: number, height: number): DOMRect {
  return {
    x: 0,
    y: 0,
    width,
    height,
    top: 0,
    right: width,
    bottom: height,
    left: 0,
    toJSON: () => ({}),
  };
}

function rectWithWidth(width: number): DOMRect {
  return rectWithSize(width, 0);
}

function renderMinimumLayoutFixture() {
  const metrics = {
    currentWideWidth: 1200,
    documentMinimumExtraWidth: 26,
    inspectorBorderWidth: 1,
    inspectorPadding: 20,
    readmeBorderWidth: 1,
    readmePadding: 12,
    readmeTextWidth: 728,
    sidebarWidth: 280,
    timelineHeight: 316,
    tooNarrowWidth: 900,
    tooShortHeight: 280,
    windowHeight: 720,
    workAreaHorizontalPadding: 32,
    workAreaVerticalPadding: 28,
  };
  const readableWidthFromMetrics =
    metrics.sidebarWidth +
    metrics.workAreaHorizontalPadding * 2 +
    (metrics.inspectorPadding + metrics.inspectorBorderWidth) * 2 +
    metrics.readmeTextWidth +
    (metrics.readmePadding + metrics.readmeBorderWidth) * 2 +
    24;
  const readableHeightFromMetrics =
    metrics.timelineHeight + metrics.workAreaVerticalPadding * 2 + 24;
  const documentMinimumWidth = readableWidthFromMetrics + metrics.documentMinimumExtraWidth;
  const window = new Window();
  const root = window.document.createElement("div");
  const sidebar = window.document.createElement("nav");
  const workArea = window.document.createElement("main");
  const timeline = window.document.createElement("section");
  const readme = window.document.createElement("textarea");

  sidebar.dataset.testid = "shell-sidebar-host";
  workArea.dataset.testid = "shell-main-host";
  timeline.dataset.testid = "timeline-view";
  readme.className = "task-readme-control-input mantine-Textarea-input";
  workArea.append(timeline);
  root.append(sidebar, workArea, readme);
  window.document.body.append(root);

  Object.defineProperty(window.document.body, "scrollWidth", {
    configurable: true,
    value: documentMinimumWidth,
  });
  sidebar.getBoundingClientRect = () => rectWithWidth(metrics.sidebarWidth);
  window.getComputedStyle = ((element: Element) => {
    if (element === workArea) {
      return {
        paddingLeft: `${metrics.workAreaHorizontalPadding}px`,
        paddingRight: `${metrics.workAreaHorizontalPadding}px`,
        paddingTop: `${metrics.workAreaVerticalPadding}px`,
        paddingBottom: `${metrics.workAreaVerticalPadding}px`,
        borderLeftWidth: "0px",
        borderRightWidth: "0px",
        borderTopWidth: "0px",
        borderBottomWidth: "0px",
      } as CSSStyleDeclaration;
    }
    if ((element as HTMLElement).classList.contains("window-minimum-inspector-probe")) {
      return {
        paddingLeft: `${metrics.inspectorPadding}px`,
        paddingRight: `${metrics.inspectorPadding}px`,
        paddingTop: `${metrics.inspectorPadding}px`,
        paddingBottom: `${metrics.inspectorPadding}px`,
        borderLeftWidth: `${metrics.inspectorBorderWidth}px`,
        borderRightWidth: `${metrics.inspectorBorderWidth}px`,
        borderTopWidth: `${metrics.inspectorBorderWidth}px`,
        borderBottomWidth: `${metrics.inspectorBorderWidth}px`,
      } as CSSStyleDeclaration;
    }
    if (element === readme) {
      return {
        font: "13px ui-monospace",
        paddingLeft: `${metrics.readmePadding}px`,
        paddingRight: `${metrics.readmePadding}px`,
        paddingTop: `${metrics.readmePadding}px`,
        paddingBottom: `${metrics.readmePadding}px`,
        borderLeftWidth: `${metrics.readmeBorderWidth}px`,
        borderRightWidth: `${metrics.readmeBorderWidth}px`,
        borderTopWidth: `${metrics.readmeBorderWidth}px`,
        borderBottomWidth: `${metrics.readmeBorderWidth}px`,
      } as CSSStyleDeclaration;
    }
    return {
      paddingLeft: "0px",
      paddingRight: "0px",
      paddingTop: "0px",
      paddingBottom: "0px",
      borderLeftWidth: "0px",
      borderRightWidth: "0px",
      borderTopWidth: "0px",
      borderBottomWidth: "0px",
    } as CSSStyleDeclaration;
  }) as typeof window.getComputedStyle;

  const originalRect = window.HTMLElement.prototype.getBoundingClientRect;
  window.HTMLElement.prototype.getBoundingClientRect = function getBoundingClientRect() {
    if (this.classList.contains("window-minimum-readme-probe")) {
      return rectWithWidth(metrics.readmeTextWidth);
    }
    if (this === timeline) {
      return rectWithSize(940, metrics.timelineHeight);
    }
    return originalRect.call(this);
  };

  const expectedMinimumWidth = Math.max(readableWidthFromMetrics, documentMinimumWidth);
  const expectedMinimumHeight = readableHeightFromMetrics;

  return { expectedMinimumHeight, expectedMinimumWidth, metrics, root };
}

test("dynamic native minimum size includes rendered sidebar, README, and timeline measurements", () => {
  const { expectedMinimumHeight, expectedMinimumWidth, root } = renderMinimumLayoutFixture();

  const size = measureRenderedMinimumWindowSize(root);

  assert.deepEqual(size, {
    height: expectedMinimumHeight,
    width: expectedMinimumWidth,
  });
});

test("syncing native minimum size applies the measured rendered minimum", async () => {
  const { expectedMinimumHeight, expectedMinimumWidth, root } = renderMinimumLayoutFixture();
  const appliedSizes: Array<{ width: number; height: number }> = [];

  const size = await syncNativeWindowMinimumSize(root, (nextSize) => {
    appliedSizes.push(nextSize);
  });

  assert.deepEqual(size, { height: expectedMinimumHeight, width: expectedMinimumWidth });
  assert.deepEqual(appliedSizes, [
    { height: expectedMinimumHeight, width: expectedMinimumWidth },
  ]);
});

test("native window policy grows a too-small current window to the rendered minimum", async () => {
  const { expectedMinimumHeight, expectedMinimumWidth, metrics } = renderMinimumLayoutFixture();
  const calls: Array<{ operation: string; size: { width: number; height: number } }> = [];

  await enforceNativeWindowMinimumSize(
    { height: expectedMinimumHeight, width: expectedMinimumWidth },
    {
      setMinimumSize: (size) => {
        calls.push({ operation: "setMinimumSize", size });
      },
      getCurrentLogicalSize: () => ({
        width: metrics.tooNarrowWidth,
        height: metrics.tooShortHeight,
      }),
      setCurrentLogicalSize: (size) => {
        calls.push({ operation: "setCurrentLogicalSize", size });
      },
    },
  );

  assert.deepEqual(calls, [
    {
      operation: "setMinimumSize",
      size: { height: expectedMinimumHeight, width: expectedMinimumWidth },
    },
    {
      operation: "setCurrentLogicalSize",
      size: { height: expectedMinimumHeight, width: expectedMinimumWidth },
    },
  ]);
});

test("native window policy leaves a wide-enough current window size unchanged", async () => {
  const { expectedMinimumHeight, expectedMinimumWidth, metrics } = renderMinimumLayoutFixture();
  const calls: Array<{ operation: string; size: { width: number; height: number } }> = [];

  await enforceNativeWindowMinimumSize(
    { height: expectedMinimumHeight, width: expectedMinimumWidth },
    {
      setMinimumSize: (size) => {
        calls.push({ operation: "setMinimumSize", size });
      },
      getCurrentLogicalSize: () => ({
        width: metrics.currentWideWidth,
        height: metrics.windowHeight,
      }),
      setCurrentLogicalSize: (size) => {
        calls.push({ operation: "setCurrentLogicalSize", size });
      },
    },
  );

  assert.deepEqual(calls, [
    {
      operation: "setMinimumSize",
      size: { height: expectedMinimumHeight, width: expectedMinimumWidth },
    },
  ]);
});
