import { ActionIcon, Box, Menu, Tree, useTree } from "@mantine/core";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type {
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
  ReactNode,
} from "react";
import type { RenderTreeNodePayload } from "@mantine/core";

import type { RememberedProjectRecord } from "./remembered-projects.ts";
import { dismissibleControlledMenuProps } from "./controlled-menu.ts";
import { SidebarSectionCreateAction } from "./sidebar-section-create-action.tsx";
import {
  PositionedTaskActionMenu,
  ProjectActionsMenu,
  SidebarEventActionMenu,
  SidebarTaskActionMenu,
  type EventAction,
  type MenuPosition,
  type TaskAction,
} from "./sidebar-action-menus.tsx";
import type { ProjectOpenState } from "./shell-types.ts";
import {
  buildSidebarNavigatorModel,
  type RememberedProjectsSidebarState,
  type SidebarNavigatorIntent,
  type SidebarNavigatorModel,
} from "./sidebar-tree-model.ts";
import {
  isTreeMenuTarget,
  keyboardMenuPositionForTreeItem,
  syncTreeItemAccessibleNames,
  treeItemForEvent,
  treeValueForEvent,
} from "./sidebar-tree-mantine-adapter.ts";
import {
  EventsIcon,
  ProjectFolderIcon,
  ProjectsMenuToggleIcon,
  TasksIcon,
} from "./sidebar-tree-menu-icons.tsx";

export type {
  RememberedProjectSidebarContents,
  RememberedProjectsSidebarState,
  SidebarNavigatorIntent,
} from "./sidebar-tree-model.ts";
export { emptyRememberedProjectsSidebarState } from "./sidebar-tree-model.ts";

export interface TaskMenuTarget {
  projectPath: string;
  taskId: string;
  position: MenuPosition;
  source: TaskMenuSource;
}

export type TaskMenuSource = "sidebar" | "timeline";

export interface EventMenuTarget { projectPath: string; eventId: string; }

export interface ProjectContextMenuTarget { projectPath: string; }

export interface SidebarNavigatorIntentContext {
  menuPosition?: MenuPosition;
  pointerEvent?: MouseEvent;
}

interface TreeRowProps {
  className: string;
  icon?: ReactNode;
  label: string;
  trailing?: ReactNode;
  active?: boolean;
  diagnostic?: boolean;
  reserveDiagnosticMarker?: boolean;
}

function TreeRow({
  className,
  icon = null,
  label,
  trailing = null,
  active = false,
  diagnostic = false,
  reserveDiagnosticMarker = false,
}: TreeRowProps) {
  const bodyClassName = [
    "projects-tree-navlink-body",
    icon ? "projects-tree-navlink-body-with-icon" : null,
    reserveDiagnosticMarker ? "projects-tree-navlink-body-with-marker" : null,
    icon && reserveDiagnosticMarker ? "projects-tree-navlink-body-with-icon-and-marker" : null,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      className={`${className} projects-tree-navlink-root`}
      data-active={active ? true : undefined}
      data-placement-status={diagnostic ? "unresolved" : undefined}
    >
      <span className={bodyClassName}>
        {icon ? (
          <span aria-hidden="true" className="projects-tree-node-icon">
            {icon}
          </span>
        ) : null}
        {diagnostic ? (
          <span aria-hidden="true" className="projects-tree-node-diagnostic-marker">
            !
          </span>
        ) : reserveDiagnosticMarker ? (
          <span aria-hidden="true" className="projects-tree-node-diagnostic-spacer" />
        ) : null}
        <span className="projects-tree-navlink-label projects-tree-node-label">
          {label}
        </span>
        {trailing ? (
          <Box className="projects-tree-node-trailing" component="span">
            {trailing}
          </Box>
        ) : null}
      </span>
    </div>
  );
}

function pointerMenuPosition(event: ReactMouseEvent<HTMLElement>): MenuPosition {
  return { x: event.clientX, y: event.clientY };
}

interface SectionActionLayout {
  top: number;
  height: number;
}

function sectionActionLayout(
  treeItem: HTMLElement,
  sidebarTop: number,
): SectionActionLayout {
  const sectionRow =
    treeItem.querySelector<HTMLElement>(".projects-tree-node-row-section") ?? treeItem;
  const bounds = sectionRow.getBoundingClientRect();
  return {
    top: bounds.top - sidebarTop,
    height: bounds.height || 24,
  };
}

function equalSectionActionLayouts(
  left: Record<string, SectionActionLayout>,
  right: Record<string, SectionActionLayout>,
): boolean {
  const leftEntries = Object.entries(left);
  const rightKeys = Object.keys(right);
  return (
    leftEntries.length === rightKeys.length &&
    leftEntries.every(([value, layout]) => {
      const nextLayout = right[value];
      return nextLayout?.top === layout.top && nextLayout.height === layout.height;
    })
  );
}

function renderSidebarTreeNode(
  payload: RenderTreeNodePayload,
  navigator: SidebarNavigatorModel,
  sidebarState: RememberedProjectsSidebarState,
  openProjectContextMenu: ProjectContextMenuTarget | null,
  openTaskMenu: TaskMenuTarget | null,
  openEventMenu: EventMenuTarget | null,
  onProjectActionMenuToggle?: (projectPath: string) => void,
  onProjectActionMenuClose?: () => void,
  onNavigatorIntent?: (intent: SidebarNavigatorIntent) => void | Promise<void>,
  onTaskAction?: (projectPath: string, taskId: string, action: TaskAction) => void | Promise<void>,
  onEventAction?: (projectPath: string, eventId: string, action: EventAction) => void | Promise<void>,
  onTaskMenuClose?: () => void,
  onEventMenuClose?: () => void,
  onNavigatorLocalIntent?: (intent: SidebarNavigatorIntent | null) => void,
) {
  const meta = navigator.nodeForValue(payload.node.value)?.meta ?? null;
  if (!meta) {
    return <div {...payload.elementProps}>{payload.node.label}</div>;
  }

  const { onClick, className, ...elementProps } = payload.elementProps;
  const labelClassName = `${className} projects-tree-label-shell project-tree-${meta.kind}`;
  const row = (() => {
    if (meta.kind === "project") {
      const isRefreshing = sidebarState.refreshInFlightProjectPaths.includes(meta.project.projectPath);
      return (
        <>
          <TreeRow
            active={payload.selected}
            className={
              meta.project.missing
                ? "projects-tree-node-row-project projects-tree-node-row-project-missing"
                : "projects-tree-node-row-project"
            }
            icon={<ProjectFolderIcon expanded={payload.expanded} />}
            label={meta.label}
            trailing={
              <ProjectActionsMenu
                project={meta.project}
                open={openProjectContextMenu?.projectPath === meta.project.projectPath}
                refreshing={isRefreshing}
                onProjectActionMenuToggle={onProjectActionMenuToggle}
                onProjectActionMenuClose={onProjectActionMenuClose}
                onProjectAction={(_projectPath, command) => {
                  const intent = navigator.projectCommandIntentForValue(payload.node.value, command);
                  if (intent) {
                    void onNavigatorIntent?.(intent);
                  }
                }}
              />
            }
          />
          {isRefreshing ? (
            <p className="project-tree-status" data-testid="remembered-project-refreshing">
              Refreshing...
            </p>
          ) : null}
        </>
      );
    }

    if (meta.kind === "section") {
      const section = meta.section === "events" ? "events" : "tasks";
      return (
        <TreeRow
          className="projects-tree-node-row-section"
          icon={section === "tasks" ? <TasksIcon /> : <EventsIcon />}
          label={meta.label}
        />
      );
    }

    if (meta.kind === "task") {
      const isOpen =
        openTaskMenu?.source === "sidebar" &&
        openTaskMenu.projectPath === meta.project.projectPath &&
        openTaskMenu.taskId === meta.itemName;
      return (
        <SidebarTaskActionMenu
          open={isOpen && Boolean(meta.itemName)}
          projectPath={meta.project.projectPath}
          taskId={meta.itemName ?? ""}
          onTaskAction={onTaskAction}
          onTaskMenuClose={onTaskMenuClose}
        >
          <TreeRow
            active={payload.selected}
            className="projects-tree-node-row-task"
            diagnostic={meta.diagnostic}
            label={meta.label}
            reserveDiagnosticMarker
          />
        </SidebarTaskActionMenu>
      );
    }

    if (meta.kind === "event") {
      const isOpen =
        openEventMenu?.projectPath === meta.project.projectPath &&
        openEventMenu.eventId === meta.itemName;
      return (
        <SidebarEventActionMenu
          open={isOpen && Boolean(meta.itemName)}
          projectPath={meta.project.projectPath}
          eventId={meta.itemName ?? ""}
          onEventAction={onEventAction}
          onEventMenuClose={onEventMenuClose}
        >
          <TreeRow
            active={payload.selected}
            className="projects-tree-node-row-event"
            diagnostic={meta.diagnostic}
            label={meta.label}
            reserveDiagnosticMarker
          />
        </SidebarEventActionMenu>
      );
    }

    return <p className="project-tree-empty">{meta.label}</p>;
  })();

  return (
    <div
      {...elementProps}
      aria-label={meta.label}
      className={labelClassName}
      onClick={(event) => {
        if (isTreeMenuTarget(event)) {
          return;
        }
        if (meta.kind === "section") {
          event.preventDefault();
          event.stopPropagation();
          onNavigatorLocalIntent?.(navigator.toggleSectionIntentForValue(payload.node.value));
          return;
        }
        onClick?.(event);
      }}
    >
      {row}
    </div>
  );
}

export interface RememberedProjectsSectionProps {
  records: RememberedProjectRecord[];
  projectState: ProjectOpenState;
  selectedTaskId: string | null;
  selectedEventId: string | null;
  sidebarState: RememberedProjectsSidebarState;
  projectsMenuOpen: boolean;
  openProjectContextMenu: ProjectContextMenuTarget | null;
  openTaskCreateMenuProjectPath: string | null;
  openTaskMenu: TaskMenuTarget | null;
  openEventMenu: EventMenuTarget | null;
  onNavigatorIntent?: (
    intent: SidebarNavigatorIntent,
    context?: SidebarNavigatorIntentContext,
  ) => void | Promise<void>;
  onProjectsMenuToggle?: () => void;
  onProjectsMenuClose?: () => void;
  onProjectsMenuAction?: (action: "new-project" | "open-existing-project") => void | Promise<void>;
  onProjectActionMenuToggle?: (projectPath: string) => void;
  onProjectActionMenuClose?: () => void;
  onTaskCreateMenuToggle?: (projectPath: string) => void;
  onTaskCreateMenuClose?: () => void;
  onTaskMenuClose?: () => void;
  onTaskAction?: (projectPath: string, taskId: string, action: TaskAction) => void | Promise<void>;
  onEventMenuClose?: () => void;
  onEventAction?: (projectPath: string, eventId: string, action: "rename" | "delete") => void | Promise<void>;
}

export function RememberedProjectsSection({
  records,
  projectState,
  selectedTaskId,
  selectedEventId,
  sidebarState,
  projectsMenuOpen,
  openProjectContextMenu,
  openTaskCreateMenuProjectPath,
  openTaskMenu,
  openEventMenu,
  onNavigatorIntent,
  onProjectsMenuToggle,
  onProjectsMenuClose,
  onProjectsMenuAction,
  onProjectActionMenuToggle,
  onProjectActionMenuClose,
  onTaskCreateMenuToggle,
  onTaskCreateMenuClose,
  onTaskMenuClose,
  onTaskAction,
  onEventMenuClose,
  onEventAction,
}: RememberedProjectsSectionProps) {
  const [sectionExpandedState, setSectionExpandedState] = useState<Record<string, boolean>>({});
  const [sectionActionLayouts, setSectionActionLayouts] = useState<
    Record<string, SectionActionLayout>
  >({});
  const treeRootRef = useRef<HTMLUListElement>(null);
  const treeModel = useMemo(
    () =>
      buildSidebarNavigatorModel({
        records,
        projectState,
        selectedTaskId,
        selectedEventId,
        sidebarState,
        sectionExpandedState,
      }),
    [records, projectState, selectedTaskId, selectedEventId, sidebarState, sectionExpandedState],
  );
  const sectionActionNodes = useMemo(
    () =>
      treeModel.data.flatMap((project) => {
        if (!treeModel.expandedState[project.value]) {
          return [];
        }
        return (project.children ?? []).filter((node) => node.meta.kind === "section");
      }),
    [treeModel],
  );

  const dispatchNavigatorIntent = (
    intent: SidebarNavigatorIntent | null,
    context?: SidebarNavigatorIntentContext,
  ) => {
    if (!intent) {
      return;
    }
    if (intent.kind === "set-section-expanded") {
      setSectionExpandedState((current) => ({ ...current, [intent.value]: intent.expanded }));
      return;
    }
    void onNavigatorIntent?.(intent, context);
  };

  const contextForNavigatorIntent = (
    intent: SidebarNavigatorIntent,
    event: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>,
    treeItem: HTMLElement,
  ): SidebarNavigatorIntentContext | undefined => {
    if (intent.kind === "open-project-context-menu" && event.type === "contextmenu") {
      return { pointerEvent: (event as ReactMouseEvent<HTMLElement>).nativeEvent };
    }
    if (intent.kind === "open-task-context-menu") {
      return {
        menuPosition:
          event.type === "contextmenu"
            ? pointerMenuPosition(event as ReactMouseEvent<HTMLElement>)
            : keyboardMenuPositionForTreeItem(treeItem),
      };
    }
    return undefined;
  };

  const tree = useTree({
    expandedState: treeModel.expandedState,
    selectedState: treeModel.selectedState,
    onNodeCollapse: (value) => {
      dispatchNavigatorIntent(treeModel.expansionIntentForValue(value, false));
    },
    onNodeExpand: (value) => {
      dispatchNavigatorIntent(treeModel.expansionIntentForValue(value, true));
    },
    onSelectedStateChange: (selectedValues) => {
      dispatchNavigatorIntent(treeModel.activationIntentForValue(selectedValues.at(-1)));
    },
  });

  useLayoutEffect(() => {
    const treeRoot = treeRootRef.current;
    syncTreeItemAccessibleNames(treeRoot, treeModel);
    if (!treeRoot) {
      return;
    }

    const sidebarBounds = treeRoot
      .closest<HTMLElement>(".remembered-projects")
      ?.getBoundingClientRect();
    const nextLayouts: Record<string, SectionActionLayout> = {};
    treeRoot.querySelectorAll<HTMLElement>("[role='treeitem'][data-value]").forEach((treeItem) => {
      const value = treeItem.dataset.value;
      const meta = treeModel.nodeForValue(value)?.meta ?? null;
      if (!value || meta?.kind !== "section") {
        return;
      }

      nextLayouts[value] = sectionActionLayout(treeItem, sidebarBounds?.top ?? 0);
    });
    setSectionActionLayouts((current) =>
      equalSectionActionLayouts(current, nextLayouts) ? current : nextLayouts,
    );
  }, [treeModel]);

  const dispatchEventNavigatorIntent = (
    intent: SidebarNavigatorIntent | null,
    event: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>,
    treeItem: HTMLElement,
  ) => {
    if (!intent) {
      return;
    }
    event.preventDefault();
    dispatchNavigatorIntent(intent, contextForNavigatorIntent(intent, event, treeItem));
  };

  return (
    <section className="remembered-projects" aria-label="Projects">
      <Menu
        closeOnItemClick={false}
        {...dismissibleControlledMenuProps(projectsMenuOpen, onProjectsMenuClose)}
        position="bottom-end"
        shadow="md"
        withinPortal={false}
      >
        <div className="projects-menu-row">
          <p className="eyebrow">Projects</p>
          <Menu.Target>
            <ActionIcon
              aria-controls="projects-menu"
              aria-expanded={projectsMenuOpen}
              aria-haspopup="menu"
              aria-label="Add project"
              className="projects-menu-toggle"
              data-testid="projects-menu-toggle"
              radius="md"
              size="sm"
              variant="subtle"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onProjectsMenuToggle?.();
              }}
            >
              <ProjectsMenuToggleIcon />
            </ActionIcon>
          </Menu.Target>
        </div>
        {projectsMenuOpen ? (
          <Menu.Dropdown
            className="projects-menu"
            id="projects-menu"
            role="menu"
            aria-label="Project actions"
          >
            <Menu.Item
              className="projects-menu-item"
              data-testid="new-project"
              aria-label="Start from scratch"
              onClick={() => {
                void onProjectsMenuAction?.("new-project");
              }}
            >
              Start from scratch
            </Menu.Item>
            <Menu.Item
              className="projects-menu-item"
              data-testid="open-existing-project"
              aria-label="Use an existing folder"
              onClick={() => {
                void onProjectsMenuAction?.("open-existing-project");
              }}
            >
              Use an existing folder
            </Menu.Item>
          </Menu.Dropdown>
        ) : null}
      </Menu>
      <Tree
        ref={treeRootRef}
        aria-label="Projects"
        className="project-tree"
        data={treeModel.data}
        data-testid="projects-tree"
        expandOnClick={false}
        expandOnSpace={false}
        levelOffset={14}
        renderNode={(payload) =>
          renderSidebarTreeNode(
            payload,
            treeModel,
            sidebarState,
            openProjectContextMenu,
            openTaskMenu,
            openEventMenu,
            onProjectActionMenuToggle,
            onProjectActionMenuClose,
            (intent) => dispatchNavigatorIntent(intent),
            onTaskAction,
            onEventAction,
            onTaskMenuClose,
            onEventMenuClose,
            (intent) => dispatchNavigatorIntent(intent),
          )
        }
        selectOnClick
        tree={tree}
        withLines
        onContextMenu={(event) => {
          if (isTreeMenuTarget(event)) {
            return;
          }
          const treeItem = treeItemForEvent(event);
          const value = treeValueForEvent(event);
          if (treeItem) {
            dispatchEventNavigatorIntent(
              treeModel.contextMenuIntentForValue(value, "pointer"),
              event,
              treeItem,
            );
          }
        }}
        onKeyDown={(event) => {
          if (isTreeMenuTarget(event)) {
            return;
          }
          const treeItem = treeItemForEvent(event);
          const value = treeValueForEvent(event);
          if (!treeItem) {
            return;
          }
          dispatchEventNavigatorIntent(
            treeModel.keyboardIntentForNode(value, event.key, event.shiftKey),
            event,
            treeItem,
          );
        }}
      />
      <div className="project-tree-section-actions-layer">
        {sectionActionNodes.map((node) => {
          const layout = sectionActionLayouts[node.value];
          const style: CSSProperties = layout
            ? { top: `${layout.top}px`, height: `${layout.height}px` }
            : { top: 0, height: 24 };
          const section = node.meta.section === "events" ? "events" : "tasks";

          return (
            <div
              className="project-tree-section-action-slot"
              data-section={section}
              key={node.value}
              style={style}
            >
              <SidebarSectionCreateAction
                project={node.meta.project}
                section={section}
                openTaskCreateMenuProjectPath={openTaskCreateMenuProjectPath}
                onTaskCreateMenuToggle={onTaskCreateMenuToggle}
                onTaskCreateMenuClose={onTaskCreateMenuClose}
                onProjectAction={(projectPath, command) => {
                  void onNavigatorIntent?.(
                    treeModel.projectCommandIntentForProject(projectPath, command),
                  );
                }}
              />
            </div>
          );
        })}
      </div>
      {treeModel.treeRecords.length ? null : (
        <p className="project-tree-empty">No remembered projects yet.</p>
      )}
      <PositionedTaskActionMenu
        openTaskMenu={openTaskMenu}
        onTaskAction={onTaskAction}
        onTaskMenuClose={onTaskMenuClose}
      />
    </section>
  );
}
