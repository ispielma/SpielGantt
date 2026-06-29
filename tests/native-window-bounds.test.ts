import assert from "node:assert/strict";
import test from "node:test";

import { Window } from "happy-dom";

import {
  clampWindowBoundsToWorkArea,
  createLocalStorageWindowBoundsStore,
  enforceNativeWindowWorkAreaBounds,
  rememberNativeWindowBounds,
  restoreNativeWindowBounds,
} from "../src/native-window-bounds.ts";

test("native window bounds clamp shrinks an oversized restored window to the current work area", () => {
  const bounds = { x: -120, y: -80, width: 2400, height: 1600 };
  const workArea = { x: 0, y: 25, width: 1440, height: 875 };

  assert.deepEqual(clampWindowBoundsToWorkArea(bounds, workArea), {
    x: 0,
    y: 25,
    width: 1440,
    height: 875,
  });
});

test("native window bounds clamp moves an offscreen restored window inside the current work area", () => {
  const bounds = { x: 1200, y: 820, width: 640, height: 360 };
  const workArea = { x: 0, y: 25, width: 1440, height: 875 };

  assert.deepEqual(clampWindowBoundsToWorkArea(bounds, workArea), {
    x: 800,
    y: 540,
    width: 640,
    height: 360,
  });
});

test("native window bounds policy corrects only restored bounds outside the current work area", async () => {
  const calls: Array<{ x: number; y: number; width: number; height: number }> = [];

  const correctedBounds = await enforceNativeWindowWorkAreaBounds({
    getCurrentPhysicalBounds: () => ({ x: -80, y: 50, width: 1600, height: 900 }),
    getCurrentMonitorWorkArea: () => ({ x: 0, y: 25, width: 1440, height: 875 }),
    setCurrentPhysicalBounds: (bounds) => {
      calls.push(bounds);
    },
  });

  assert.deepEqual(correctedBounds, { x: 0, y: 25, width: 1440, height: 875 });
  assert.deepEqual(calls, [{ x: 0, y: 25, width: 1440, height: 875 }]);
});

test("native window bounds policy leaves bounds unchanged when they fit the current work area", async () => {
  const calls: Array<{ x: number; y: number; width: number; height: number }> = [];

  const correctedBounds = await enforceNativeWindowWorkAreaBounds({
    getCurrentPhysicalBounds: () => ({ x: 100, y: 80, width: 1200, height: 720 }),
    getCurrentMonitorWorkArea: () => ({ x: 0, y: 25, width: 1440, height: 875 }),
    setCurrentPhysicalBounds: (bounds) => {
      calls.push(bounds);
    },
  });

  assert.deepEqual(correctedBounds, { x: 100, y: 80, width: 1200, height: 720 });
  assert.deepEqual(calls, []);
});

test("native window bounds store persists restored bounds in frontend storage", async () => {
  const window = new Window();
  const store = createLocalStorageWindowBoundsStore(window.localStorage);

  await rememberNativeWindowBounds(
    { getCurrentPhysicalBounds: () => ({ x: -80, y: 50, width: 1600, height: 900 }) },
    store,
  );
  const restoredBounds = await restoreNativeWindowBounds(
    {
      getCurrentPhysicalBounds: () => ({ x: 20, y: 50, width: 800, height: 500 }),
      getCurrentMonitorWorkArea: () => ({ x: 0, y: 25, width: 1440, height: 875 }),
      setCurrentPhysicalBounds: () => {},
    },
    store,
  );

  assert.deepEqual(restoredBounds, { x: 0, y: 25, width: 1440, height: 875 });
  assert.deepEqual(store.readBounds(), { x: 0, y: 25, width: 1440, height: 875 });
});
