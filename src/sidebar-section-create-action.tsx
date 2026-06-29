import { ActionIcon, Menu } from "@mantine/core";
import type { KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent } from "react";

import { dismissibleControlledMenuProps } from "./controlled-menu.ts";
import {
  projectActionCommandForId,
  type ProjectActionCommand,
} from "./project-actions.ts";
import type { SidebarProjectTreeRecord } from "./sidebar-tree-model.ts";
import { SectionCreateIcon } from "./sidebar-tree-menu-icons.tsx";

interface SidebarSectionCreateActionProps {
  project: SidebarProjectTreeRecord;
  section: "tasks" | "events";
  openTaskCreateMenuProjectPath: string | null;
  onTaskCreateMenuToggle?: (projectPath: string) => void;
  onTaskCreateMenuClose?: () => void;
  onProjectAction?: (projectPath: string, command: ProjectActionCommand) => void | Promise<void>;
}

function isKeyboardActivationKey(key: string): boolean {
  return key === "Enter" || key === " ";
}

export function SidebarSectionCreateAction({
  project,
  section,
  openTaskCreateMenuProjectPath,
  onTaskCreateMenuToggle,
  onTaskCreateMenuClose,
  onProjectAction,
}: SidebarSectionCreateActionProps) {
  const stopSectionActionEvent = (
    event: ReactKeyboardEvent<HTMLButtonElement> | ReactMouseEvent<HTMLButtonElement>,
  ) => {
    event.preventDefault();
    event.stopPropagation();
  };

  if (section === "events") {
    const createEvent = () => {
      void onProjectAction?.(project.projectPath, projectActionCommandForId("create-event"));
    };

    return (
      <ActionIcon
        aria-label={`Create event in ${project.projectName}`}
        className="project-tree-section-create-toggle"
        radius="md"
        size="sm"
        variant="subtle"
        onClick={(event) => {
          stopSectionActionEvent(event);
          createEvent();
        }}
        onKeyDown={(event) => {
          if (!isKeyboardActivationKey(event.key)) {
            return;
          }
          stopSectionActionEvent(event);
          createEvent();
        }}
      >
        <SectionCreateIcon />
      </ActionIcon>
    );
  }

  const isOpen = openTaskCreateMenuProjectPath === project.projectPath;
  const toggleTaskCreateMenu = () => {
    onTaskCreateMenuToggle?.(project.projectPath);
  };

  return (
    <Menu
      closeOnItemClick={false}
      {...dismissibleControlledMenuProps(isOpen, onTaskCreateMenuClose)}
      position="bottom-end"
      shadow="md"
      withinPortal={false}
    >
      <Menu.Target>
        <ActionIcon
          aria-expanded={isOpen}
          aria-haspopup="menu"
          aria-label={`Task creation options for ${project.projectName}`}
          className="project-tree-section-create-toggle"
          radius="md"
          size="sm"
          variant="subtle"
          onClick={(event) => {
            stopSectionActionEvent(event);
            toggleTaskCreateMenu();
          }}
          onKeyDown={(event) => {
            if (!isKeyboardActivationKey(event.key)) {
              return;
            }
            stopSectionActionEvent(event);
            toggleTaskCreateMenu();
          }}
        >
          <SectionCreateIcon />
        </ActionIcon>
      </Menu.Target>
      {isOpen ? (
        <Menu.Dropdown role="menu" aria-label={`Task creation options for ${project.projectName}`}>
          <Menu.Item
            aria-label={`Create task in ${project.projectName}`}
            onClick={() =>
              void onProjectAction?.(project.projectPath, projectActionCommandForId("create-task"))
            }
          >
            Create task
          </Menu.Item>
          <Menu.Item
            aria-label={`Create task from folder in ${project.projectName}`}
            onClick={() =>
              void onProjectAction?.(
                project.projectPath,
                projectActionCommandForId("create-task-from-folder"),
              )
            }
          >
            Create task from folder...
          </Menu.Item>
        </Menu.Dropdown>
      ) : null}
    </Menu>
  );
}
