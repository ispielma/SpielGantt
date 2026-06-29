import type { ProjectContextMenuTarget } from "./sidebar-tree.tsx";
import {
  popupProjectActionMenu,
} from "./shell-tauri-menu-adapter.ts";
import type { ProjectActionCommand, ProjectActionEntry } from "./project-actions.ts";

export type { ProjectActionCommand, ProjectActionEntry } from "./project-actions.ts";

interface ProjectActionMenuBindings {
  root: HTMLElement;
  getOpenProjectContextMenu: () => ProjectContextMenuTarget | null;
  setOpenProjectContextMenu: (target: ProjectContextMenuTarget | null) => void;
  dismissForProjectActions: () => void;
  projectActionMenuEntriesForProject: (projectPath: string) => ProjectActionEntry[];
  runProjectCommand: (projectPath: string, command: ProjectActionCommand) => Promise<void>;
  render: () => void;
}

export function openProjectActionsFallbackMenu(
  bindings: ProjectActionMenuBindings,
  projectPath: string,
) {
  bindings.dismissForProjectActions();
  bindings.setOpenProjectContextMenu({ projectPath });
  bindings.render();
}

export async function popupNativeProjectActionMenuForProject(
  bindings: ProjectActionMenuBindings,
  projectPath: string,
  event: MouseEvent,
) {
  bindings.dismissForProjectActions();
  bindings.setOpenProjectContextMenu(null);
  const opened = await popupProjectActionMenu(
    { x: event.clientX, y: event.clientY },
    bindings.projectActionMenuEntriesForProject(projectPath),
    (command) => {
      void bindings.runProjectCommand(projectPath, command);
    },
  );

  if (opened) {
    bindings.render();
    return;
  }

  openProjectActionsFallbackMenu(bindings, projectPath);
}
