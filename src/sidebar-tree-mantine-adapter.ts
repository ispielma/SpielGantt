import type { KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent } from "react";

import type { SidebarNavigatorModel } from "./sidebar-tree-model.ts";
import type { MenuPosition } from "./sidebar-action-menus.tsx";

export function syncTreeItemAccessibleNames(
  root: HTMLElement | null,
  navigator: SidebarNavigatorModel,
) {
  // Mantine Tree does not expose props for its outer treeitem element; keep
  // public accessible names stable while rendering row content declaratively.
  root?.querySelectorAll<HTMLElement>("[role='treeitem'][data-value]").forEach((treeItem) => {
    const value = treeItem.dataset.value;
    const meta = value ? navigator.nodeForValue(value)?.meta ?? null : null;
    if (!meta) {
      return;
    }

    treeItem.setAttribute("aria-label", meta.accessibleName);
    treeItem.removeAttribute("title");
  });
}

export function isTreeMenuTarget(
  event: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>,
) {
  const target = event.target as Element | null;
  return Boolean(target?.closest("[role='menu'], button[aria-haspopup='menu']"));
}

export function treeItemForEvent(
  event: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>,
): HTMLElement | null {
  const target = event.target as Element | null;
  return target?.closest<HTMLElement>("[role='treeitem'][data-value]") ?? null;
}

export function treeValueForEvent(
  event: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>,
): string | undefined {
  return treeItemForEvent(event)?.dataset.value;
}

export function keyboardMenuPositionForTreeItem(treeItem: HTMLElement): MenuPosition {
  const bounds = treeItem.getBoundingClientRect();
  return {
    x: bounds.left + Math.min(bounds.width, 24),
    y: bounds.top + Math.min(bounds.height, 24),
  };
}
