import type {
  EventMenuTarget,
  ProjectContextMenuTarget,
  TaskMenuSource,
  TaskMenuTarget,
} from "./sidebar-tree.tsx";
import type { MenuPosition } from "./sidebar-action-menus.tsx";

export interface ShellMenuSnapshot {
  projectsMenuOpen: boolean;
  openProjectContextMenu: ProjectContextMenuTarget | null;
  openTaskCreateMenuProjectPath: string | null;
  openTaskMenu: TaskMenuTarget | null;
  openEventMenu: EventMenuTarget | null;
}

export class ShellMenuStateController {
  private projectsMenuOpen = false;
  private openTaskCreateMenuProjectPath: string | null = null;
  private openTaskMenu: TaskMenuTarget | null = null;
  private openEventMenu: EventMenuTarget | null = null;
  private openProjectContextMenu: ProjectContextMenuTarget | null = null;

  getSnapshot(): ShellMenuSnapshot {
    return {
      projectsMenuOpen: this.projectsMenuOpen,
      openProjectContextMenu: this.openProjectContextMenu,
      openTaskCreateMenuProjectPath: this.openTaskCreateMenuProjectPath,
      openTaskMenu: this.openTaskMenu,
      openEventMenu: this.openEventMenu,
    };
  }

  setProjectsMenuOpen(open: boolean): void {
    this.projectsMenuOpen = open;
  }

  toggleProjectsMenu(): boolean {
    this.projectsMenuOpen = !this.projectsMenuOpen;
    this.closeTaskCreateAndContextMenus();
    return this.projectsMenuOpen;
  }

  closeProjectsMenu(): boolean {
    if (!this.projectsMenuOpen) {
      return false;
    }
    this.projectsMenuOpen = false;
    return true;
  }

  closeContextMenus(): void {
    this.openTaskMenu = null;
    this.openEventMenu = null;
    this.openProjectContextMenu = null;
  }

  closeTaskAndEventMenus(): void {
    this.openTaskMenu = null;
    this.openEventMenu = null;
  }

  closeTaskCreateAndContextMenus(): void {
    this.openTaskCreateMenuProjectPath = null;
    this.closeContextMenus();
  }

  closeProjectMenus(): void {
    this.projectsMenuOpen = false;
    this.closeTaskCreateAndContextMenus();
  }

  closeTaskCreateMenu(): boolean {
    if (!this.openTaskCreateMenuProjectPath) {
      return false;
    }
    this.openTaskCreateMenuProjectPath = null;
    return true;
  }

  toggleTaskCreateMenu(projectPath: string): void {
    this.projectsMenuOpen = false;
    this.closeContextMenus();
    this.openTaskCreateMenuProjectPath =
      this.openTaskCreateMenuProjectPath === projectPath ? null : projectPath;
  }

  openTaskContextMenu(
    projectPath: string,
    taskId: string,
    position: MenuPosition,
    source: TaskMenuSource = "sidebar",
  ): void {
    this.projectsMenuOpen = false;
    this.openProjectContextMenu = null;
    this.openTaskCreateMenuProjectPath = null;
    this.openTaskMenu = { projectPath, taskId, position, source };
    this.openEventMenu = null;
  }

  closeTaskContextMenu(): boolean {
    if (!this.openTaskMenu) {
      return false;
    }
    this.openTaskMenu = null;
    return true;
  }

  openEventContextMenu(projectPath: string, eventId: string): void {
    this.projectsMenuOpen = false;
    this.openProjectContextMenu = null;
    this.openTaskCreateMenuProjectPath = null;
    this.openTaskMenu = null;
    this.openEventMenu = { projectPath, eventId };
  }

  closeEventContextMenu(): boolean {
    if (!this.openEventMenu) {
      return false;
    }
    this.openEventMenu = null;
    return true;
  }

  closeProjectActionMenu(): boolean {
    if (!this.openProjectContextMenu) {
      return false;
    }
    this.openProjectContextMenu = null;
    return true;
  }

  toggleProjectActionMenu(projectPath: string): "open" | "closed" {
    this.openTaskCreateMenuProjectPath = null;
    if (this.openProjectContextMenu?.projectPath === projectPath) {
      this.openProjectContextMenu = null;
      return "closed";
    }
    return "open";
  }

  getOpenProjectContextMenu(): ProjectContextMenuTarget | null {
    return this.openProjectContextMenu;
  }

  setOpenProjectContextMenu(target: ProjectContextMenuTarget | null): void {
    this.openProjectContextMenu = target;
  }

  dismissForProjectActions(): void {
    this.projectsMenuOpen = false;
    this.openTaskCreateMenuProjectPath = null;
    this.openTaskMenu = null;
  }
}
