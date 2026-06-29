import { useEffect, type RefObject } from "react";

import { startTauriNativeWindowBoundsSession } from "./native-window-bounds.ts";
import { syncNativeWindowMinimumSize } from "./native-window-layout.ts";

export function useNativeWindowBounds(
  shellRootRef: RefObject<HTMLElement | null>,
  layoutDeps: readonly unknown[],
): void {
  useEffect(() => {
    const root = shellRootRef.current;
    if (!root?.ownerDocument.defaultView) {
      return;
    }

    let cleanupBoundsSession: (() => void) | null = null;
    let cancelled = false;
    void startTauriNativeWindowBoundsSession().then((cleanup) => {
      if (cancelled) {
        cleanup();
        return;
      }
      cleanupBoundsSession = cleanup;
    });

    return () => {
      cancelled = true;
      cleanupBoundsSession?.();
    };
  }, [shellRootRef]);

  useEffect(() => {
    const root = shellRootRef.current;
    const ownerWindow = root?.ownerDocument.defaultView;
    if (!root || !ownerWindow) {
      return;
    }

    let cancelled = false;
    const syncMinimumSize = () => {
      if (!cancelled) {
        void syncNativeWindowMinimumSize(root);
      }
    };
    const frame = ownerWindow.requestAnimationFrame(syncMinimumSize);
    void root.ownerDocument.fonts?.ready.then(syncMinimumSize);

    return () => {
      cancelled = true;
      ownerWindow.cancelAnimationFrame(frame);
    };
  }, [shellRootRef, ...layoutDeps]);
}
