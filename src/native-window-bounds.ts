export type NativeWindowPhysicalBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type NativeWindowBoundsGeometry = {
  getCurrentPhysicalBounds: () => Promise<NativeWindowPhysicalBounds> | NativeWindowPhysicalBounds;
  getCurrentMonitorWorkArea: () =>
    | Promise<NativeWindowPhysicalBounds | null>
    | NativeWindowPhysicalBounds
    | null;
  setCurrentPhysicalBounds: (bounds: NativeWindowPhysicalBounds) => Promise<void> | void;
};

type NativeWindowBoundsStore = {
  readBounds: () => NativeWindowPhysicalBounds | null;
  writeBounds: (bounds: NativeWindowPhysicalBounds) => void;
};

const WINDOW_BOUNDS_STORAGE_KEY = "spielgantt.nativeWindowBounds.v1";

function isFiniteBound(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isPhysicalBounds(value: unknown): value is NativeWindowPhysicalBounds {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Partial<NativeWindowPhysicalBounds>;
  return (
    isFiniteBound(candidate.x) &&
    isFiniteBound(candidate.y) &&
    isFiniteBound(candidate.width) &&
    isFiniteBound(candidate.height) &&
    candidate.width > 0 &&
    candidate.height > 0
  );
}

function clampValue(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function samePhysicalBounds(
  left: NativeWindowPhysicalBounds,
  right: NativeWindowPhysicalBounds,
): boolean {
  return (
    left.x === right.x &&
    left.y === right.y &&
    left.width === right.width &&
    left.height === right.height
  );
}

export function clampWindowBoundsToWorkArea(
  bounds: NativeWindowPhysicalBounds,
  workArea: NativeWindowPhysicalBounds,
): NativeWindowPhysicalBounds {
  const width = Math.min(bounds.width, workArea.width);
  const height = Math.min(bounds.height, workArea.height);
  const maxX = workArea.x + workArea.width - width;
  const maxY = workArea.y + workArea.height - height;

  return {
    x: clampValue(bounds.x, workArea.x, maxX),
    y: clampValue(bounds.y, workArea.y, maxY),
    width,
    height,
  };
}

export function createLocalStorageWindowBoundsStore(
  storage: Storage,
  storageKey = WINDOW_BOUNDS_STORAGE_KEY,
): NativeWindowBoundsStore {
  return {
    readBounds: () => {
      try {
        const parsed = JSON.parse(storage.getItem(storageKey) ?? "null");
        return isPhysicalBounds(parsed) ? parsed : null;
      } catch {
        return null;
      }
    },
    writeBounds: (bounds) => {
      storage.setItem(storageKey, JSON.stringify(bounds));
    },
  };
}

export async function enforceNativeWindowWorkAreaBounds(
  geometry: NativeWindowBoundsGeometry,
  requestedBounds: NativeWindowPhysicalBounds | null = null,
): Promise<NativeWindowPhysicalBounds | null> {
  const workArea = await geometry.getCurrentMonitorWorkArea();
  if (!workArea) {
    return null;
  }

  const bounds = requestedBounds ?? (await geometry.getCurrentPhysicalBounds());
  const correctedBounds = clampWindowBoundsToWorkArea(bounds, workArea);
  if (!samePhysicalBounds(await geometry.getCurrentPhysicalBounds(), correctedBounds)) {
    await geometry.setCurrentPhysicalBounds(correctedBounds);
  }
  return correctedBounds;
}

export async function restoreNativeWindowBounds(
  geometry: NativeWindowBoundsGeometry,
  store: NativeWindowBoundsStore,
): Promise<NativeWindowPhysicalBounds | null> {
  const restoredBounds = await enforceNativeWindowWorkAreaBounds(
    geometry,
    store.readBounds(),
  );
  if (restoredBounds) {
    store.writeBounds(restoredBounds);
  }
  return restoredBounds;
}

export async function rememberNativeWindowBounds(
  geometry: Pick<NativeWindowBoundsGeometry, "getCurrentPhysicalBounds">,
  store: NativeWindowBoundsStore,
): Promise<void> {
  store.writeBounds(await geometry.getCurrentPhysicalBounds());
}

export async function startTauriNativeWindowBoundsSession(): Promise<() => void> {
  try {
    const { PhysicalPosition, PhysicalSize, currentMonitor, getCurrentWindow } = await import(
      "@tauri-apps/api/window"
    );
    const storage = globalThis.window?.localStorage;
    if (!storage) {
      return () => {};
    }

    const appWindow = getCurrentWindow();
    const store = createLocalStorageWindowBoundsStore(storage);
    const geometry: NativeWindowBoundsGeometry = {
      getCurrentPhysicalBounds: async () => {
        const position = await appWindow.outerPosition();
        const size = await appWindow.outerSize();
        return {
          x: position.x,
          y: position.y,
          width: size.width,
          height: size.height,
        };
      },
      getCurrentMonitorWorkArea: async () => {
        const monitor = await currentMonitor();
        if (!monitor) {
          return null;
        }
        return {
          x: monitor.workArea.position.x,
          y: monitor.workArea.position.y,
          width: monitor.workArea.size.width,
          height: monitor.workArea.size.height,
        };
      },
      setCurrentPhysicalBounds: async (nextBounds) => {
        await appWindow.setSize(new PhysicalSize(nextBounds.width, nextBounds.height));
        await appWindow.setPosition(new PhysicalPosition(nextBounds.x, nextBounds.y));
      },
    };

    await restoreNativeWindowBounds(geometry, store);
    const persistBounds = () => {
      void rememberNativeWindowBounds(geometry, store);
    };
    const unlistenMoved = await appWindow.onMoved(persistBounds);
    const unlistenResized = await appWindow.onResized(persistBounds);

    return () => {
      unlistenMoved();
      unlistenResized();
    };
  } catch {
    return () => {};
  }
}
