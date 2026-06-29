import { ActionIcon, Menu } from "@mantine/core";
import type { ReactNode } from "react";

import { dismissibleControlledMenuProps } from "./controlled-menu.ts";
import {
  projectActionEntriesForProject,
  type ProjectActionCommand,
  type ProjectActionId,
} from "./project-actions.ts";
import {
  MenuCreateEventIcon,
  MenuCreateTaskIcon,
  MenuDeleteProjectIcon,
  MenuOpenProjectIcon,
  MenuRefreshIcon,
  MenuRevealInFinderIcon,
  ProjectActionsToggleIcon,
} from "./sidebar-tree-menu-icons.tsx";
import type { SidebarProjectTreeRecord } from "./sidebar-tree-model.ts";

export interface MenuPosition {
  x: number;
  y: number;
}

export type TaskAction = "open-folder" | "add-before" | "add-after" | "rename" | "delete";
export type EventAction = "rename" | "delete";

interface SidebarLeafAction<Action extends string> {
  id: Action;
  label: string;
  ariaLabel: (itemName: string) => string;
}

interface ControlledLeafActionMenuProps<Action extends string> {
  kind: SidebarLeafKind;
  open: boolean;
  projectPath: string;
  itemName: string;
  actions: SidebarLeafAction<Action>[];
  closeOnItemClick?: boolean;
  targetClassName?: string;
  onAction?: (projectPath: string, itemName: string, action: Action) => void | Promise<void>;
  onClose?: () => void;
  children: ReactNode;
}

function taskMenuId(projectPath: string, taskId: string): string {
  return `task-actions-${encodeURIComponent(projectPath)}-${encodeURIComponent(taskId)}`;
}

function eventMenuId(projectPath: string, eventId: string): string {
  return `event-actions-${encodeURIComponent(projectPath)}-${encodeURIComponent(eventId)}`;
}

function projectActionsMenuId(projectPath: string): string {
  return `project-actions-menu-${encodeURIComponent(projectPath)}`;
}

const projectActionMenuPresentation: Record<
  ProjectActionId,
  {
    icon: ReactNode;
  }
> = {
  "refresh-project": {
    icon: <MenuRefreshIcon />,
  },
  "open-project-folder": {
    icon: <MenuRevealInFinderIcon />,
  },
  "create-task": {
    icon: <MenuCreateTaskIcon />,
  },
  "create-task-from-folder": {
    icon: <MenuCreateTaskIcon />,
  },
  "create-event": {
    icon: <MenuCreateEventIcon />,
  },
  "find-project": {
    icon: <MenuOpenProjectIcon />,
  },
  "remove-project": {
    icon: <MenuRevealInFinderIcon />,
  },
  "delete-project": {
    icon: <MenuDeleteProjectIcon />,
  },
};

function renderProjectActionMenuItems(
  project: SidebarProjectTreeRecord,
  refreshing: boolean,
  onProjectAction?: (projectPath: string, command: ProjectActionCommand) => void | Promise<void>,
) {
  return projectActionEntriesForProject({
    projectPath: project.projectPath,
    projectName: project.projectName,
    missing: project.missing,
    refreshing,
  }).map((entry, index) => {
    if (entry.kind === "separator") {
      return <Menu.Divider key={`separator-${index}`} />;
    }

    const presentation = projectActionMenuPresentation[entry.id];
    return (
      <Menu.Item
        key={entry.id}
        aria-label={entry.ariaLabel}
        className="project-actions-menu-item"
        data-project-action={entry.id}
        disabled={entry.disabled}
        leftSection={presentation.icon}
        onClick={() => {
          if (!entry.disabled) {
            void onProjectAction?.(project.projectPath, entry.command);
          }
        }}
      >
        {entry.label}
      </Menu.Item>
    );
  });
}

export function ProjectActionsMenu({
  project,
  open,
  refreshing = false,
  onProjectActionMenuToggle,
  onProjectActionMenuClose,
  onProjectAction,
}: {
  project: SidebarProjectTreeRecord;
  open: boolean;
  refreshing?: boolean;
  onProjectActionMenuToggle?: (projectPath: string) => void;
  onProjectActionMenuClose?: () => void;
  onProjectAction?: (projectPath: string, command: ProjectActionCommand) => void | Promise<void>;
}) {
  return (
    <Menu
      closeOnItemClick={false}
      {...dismissibleControlledMenuProps(open, onProjectActionMenuClose)}
      position="bottom-end"
      portalProps={{ target: ".app-shell" }}
      shadow="md"
      withinPortal
    >
      <Menu.Target>
        <ActionIcon
          aria-controls={projectActionsMenuId(project.projectPath)}
          aria-expanded={open}
          aria-haspopup="menu"
          aria-label={`Project actions for ${project.projectName}`}
          className="project-actions-toggle"
          data-project-path={project.projectPath}
          radius="md"
          size="sm"
          variant="subtle"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onProjectActionMenuToggle?.(project.projectPath);
          }}
        >
          <ProjectActionsToggleIcon />
        </ActionIcon>
      </Menu.Target>
      {open ? (
        <Menu.Dropdown
          className="project-actions-menu"
          data-project-path={project.projectPath}
          id={projectActionsMenuId(project.projectPath)}
          role="menu"
          aria-label={`Project actions for ${project.projectName}`}
        >
          {renderProjectActionMenuItems(project, refreshing, onProjectAction)}
        </Menu.Dropdown>
      ) : null}
    </Menu>
  );
}

const taskMenuActions: SidebarLeafAction<TaskAction>[] = [
  {
    id: "open-folder",
    label: "Open Task Folder",
    ariaLabel: (taskId) => `Open task folder ${taskId}`,
  },
  {
    id: "add-before",
    label: "Add before...",
    ariaLabel: (taskId) => `Add before ${taskId}`,
  },
  {
    id: "add-after",
    label: "Add after...",
    ariaLabel: (taskId) => `Add after ${taskId}`,
  },
  {
    id: "rename",
    label: "Rename Task...",
    ariaLabel: (taskId) => `Rename task ${taskId}`,
  },
  {
    id: "delete",
    label: "Delete Task...",
    ariaLabel: (taskId) => `Delete task ${taskId}`,
  },
];

const eventMenuActions: SidebarLeafAction<EventAction>[] = [
  {
    id: "rename",
    label: "Rename Event...",
    ariaLabel: (eventId) => `Rename event ${eventId}`,
  },
  {
    id: "delete",
    label: "Delete Event...",
    ariaLabel: (eventId) => `Delete event ${eventId}`,
  },
];

type SidebarLeafKind = "task" | "event";

function sidebarLeafMenuId(kind: SidebarLeafKind, projectPath: string, itemName: string): string {
  return kind === "task" ? taskMenuId(projectPath, itemName) : eventMenuId(projectPath, itemName);
}

function sidebarLeafMenuData(kind: SidebarLeafKind, action: string, itemName: string) {
  return kind === "task"
    ? { "data-task-context-action": action, "data-task-id": itemName }
    : { "data-event-context-action": action, "data-event-id": itemName };
}

function ControlledLeafActionMenu<Action extends string>({
  kind,
  open,
  projectPath,
  itemName,
  actions,
  closeOnItemClick,
  targetClassName = "projects-tree-menu-target",
  onAction,
  onClose,
  children,
}: ControlledLeafActionMenuProps<Action>) {
  const itemTypeLabel = kind === "task" ? "Task" : "Event";

  return (
    <Menu
      closeOnItemClick={closeOnItemClick}
      {...dismissibleControlledMenuProps(open, onClose)}
      position="bottom-start"
      shadow="md"
      withinPortal={false}
    >
      <Menu.Target>
        <div className={targetClassName}>{children}</div>
      </Menu.Target>
      {open ? (
        <Menu.Dropdown
          className={`${kind}-menu-panel`}
          id={sidebarLeafMenuId(kind, projectPath, itemName)}
          role="menu"
          aria-label={`${itemTypeLabel} actions for ${itemName}`}
        >
          {actions.map((action) => (
            <Menu.Item
              key={action.id}
              className="context-menu-item"
              data-project-path={projectPath}
              aria-label={action.ariaLabel(itemName)}
              onClick={(event) => {
                event.stopPropagation();
                void onAction?.(projectPath, itemName, action.id);
              }}
              {...sidebarLeafMenuData(kind, action.id, itemName)}
            >
              {action.label}
            </Menu.Item>
          ))}
        </Menu.Dropdown>
      ) : null}
    </Menu>
  );
}

export function SidebarTaskActionMenu({
  open,
  projectPath,
  taskId,
  onTaskAction,
  onTaskMenuClose,
  children,
}: {
  open: boolean;
  projectPath: string;
  taskId: string;
  onTaskAction?: (projectPath: string, taskId: string, action: TaskAction) => void | Promise<void>;
  onTaskMenuClose?: () => void;
  children: ReactNode;
}) {
  return (
    <ControlledLeafActionMenu
      kind="task"
      open={open}
      projectPath={projectPath}
      itemName={taskId}
      actions={taskMenuActions}
      closeOnItemClick={false}
      onAction={onTaskAction}
      onClose={onTaskMenuClose}
    >
      {children}
    </ControlledLeafActionMenu>
  );
}

export function SidebarEventActionMenu({
  open,
  projectPath,
  eventId,
  onEventAction,
  onEventMenuClose,
  children,
}: {
  open: boolean;
  projectPath: string;
  eventId: string;
  onEventAction?: (projectPath: string, eventId: string, action: EventAction) => void | Promise<void>;
  onEventMenuClose?: () => void;
  children: ReactNode;
}) {
  return (
    <ControlledLeafActionMenu
      kind="event"
      open={open}
      projectPath={projectPath}
      itemName={eventId}
      actions={eventMenuActions}
      closeOnItemClick={false}
      onAction={onEventAction}
      onClose={onEventMenuClose}
    >
      {children}
    </ControlledLeafActionMenu>
  );
}

export function PositionedTaskActionMenu({
  openTaskMenu,
  onTaskAction,
  onTaskMenuClose,
}: {
  openTaskMenu:
    | {
        projectPath: string;
        taskId: string;
        position: MenuPosition;
        source: "sidebar" | "timeline";
      }
    | null;
  onTaskAction?: (projectPath: string, taskId: string, action: TaskAction) => void | Promise<void>;
  onTaskMenuClose?: () => void;
}) {
  if (!openTaskMenu || openTaskMenu.source !== "timeline") {
    return null;
  }

  const { projectPath, taskId, position } = openTaskMenu;

  return (
    <div
      className="task-menu-anchor"
      style={{ left: position.x, top: position.y }}
      data-testid="task-menu-anchor"
    >
      <ControlledLeafActionMenu
        kind="task"
        open
        projectPath={projectPath}
        itemName={taskId}
        actions={taskMenuActions}
        targetClassName="task-menu-target"
        onAction={onTaskAction}
        onClose={onTaskMenuClose}
      >
        <span aria-hidden="true" />
      </ControlledLeafActionMenu>
    </div>
  );
}
