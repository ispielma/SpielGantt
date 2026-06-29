import { isTauri } from "@tauri-apps/api/core";
import { LogicalPosition } from "@tauri-apps/api/dpi";

import type {
  ProjectActionCommand,
  ProjectActionEntry,
  ProjectActionId,
} from "./project-actions.ts";

export type { ProjectActionCommand, ProjectActionEntry, ProjectActionId };

export async function popupProjectActionMenu(
  position: { x: number; y: number },
  options: ProjectActionEntry[],
  onSelect: (command: ProjectActionCommand) => void,
): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }

  try {
    const { Menu, PredefinedMenuItem } = await import("@tauri-apps/api/menu");
    const menu = await Menu.new({
      items: await Promise.all(
        options.map((option) => {
          if (option.kind === "separator") {
            return PredefinedMenuItem.new({ item: "Separator" });
          }

          return {
            id: option.id,
            text: option.label,
            enabled: !option.disabled,
            action: (id: string) => {
              const selectedOption = options.find(
                (entry) => entry.kind === "action" && entry.id === (id as ProjectActionId),
              );
              if (selectedOption?.kind === "action" && !selectedOption.disabled) {
                onSelect(selectedOption.command);
              }
            },
          };
        }),
      ),
    });

    try {
      await menu.popup(new LogicalPosition(position));
    } finally {
      await menu.close().catch(() => {});
    }

    return true;
  } catch {
    return false;
  }
}
