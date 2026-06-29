import type { RememberedProjectRecord } from "./remembered-projects.ts";
import { basenameFromPath } from "./project-tree-dom.ts";
import type { ProjectActionCommand } from "./project-actions.ts";
import type { ProjectOpenState } from "./shell-types.ts";
import {
  eventPlacementStatus,
  taskDeterminationMessages,
  taskDeterminationStatus,
} from "./workflow-node-diagnostics.ts";

export interface RememberedProjectsSidebarState {
  refreshInFlightProjectPaths: string[];
  scanFailures: Record<string, string>;
  projectContents: Record<string, RememberedProjectSidebarContents>;
}

export interface RememberedProjectSidebarContents {
  taskNames: string[];
  eventNames: string[];
}

export const emptyRememberedProjectsSidebarState: RememberedProjectsSidebarState = {
  refreshInFlightProjectPaths: [],
  scanFailures: {},
  projectContents: {},
};

export interface SidebarProjectTreeRecord {
  projectPath: string;
  projectName: string;
  expanded: boolean;
  active: boolean;
  taskNames: string[];
  eventNames: string[];
  diagnosticTaskNames: string[];
  diagnosticEventNames: string[];
  missing: boolean;
}

export type SidebarTreeNodeKind = "project" | "section" | "task" | "event" | "empty";
export type SidebarSectionKind = "tasks" | "events";
export type SidebarContextMenuSource = "pointer" | "keyboard";

export interface SidebarTreeNodeMeta {
  kind: SidebarTreeNodeKind;
  label: string;
  accessibleName: string;
  project: SidebarProjectTreeRecord;
  section?: SidebarSectionKind;
  itemName?: string;
  diagnostic?: boolean;
}

export interface SidebarNavigatorNode {
  label: string;
  value: string;
  meta: SidebarTreeNodeMeta;
  selected: boolean;
  expanded?: boolean;
  children?: SidebarNavigatorNode[];
}

export type SidebarNavigatorIntent =
  | { kind: "activate-project"; projectPath: string }
  | { kind: "select-task"; projectPath: string; taskId: string }
  | { kind: "select-event"; projectPath: string; eventId: string }
  | { kind: "toggle-project-expansion"; projectPath: string }
  | { kind: "set-section-expanded"; value: string; expanded: boolean }
  | { kind: "open-project-context-menu"; projectPath: string; source: SidebarContextMenuSource }
  | {
      kind: "open-task-context-menu";
      projectPath: string;
      taskId: string;
      source: SidebarContextMenuSource;
    }
  | {
      kind: "open-event-context-menu";
      projectPath: string;
      eventId: string;
      source: SidebarContextMenuSource;
    }
  | { kind: "run-project-command"; projectPath: string; command: ProjectActionCommand };

export interface SidebarNavigatorInput {
  records: RememberedProjectRecord[];
  projectState: ProjectOpenState;
  selectedTaskId: string | null;
  selectedEventId: string | null;
  sidebarState: RememberedProjectsSidebarState;
  sectionExpandedState?: Record<string, boolean>;
}

export interface SidebarNavigatorModel {
  nodes: SidebarNavigatorNode[];
  data: SidebarNavigatorNode[];
  treeRecords: SidebarProjectTreeRecord[];
  expandedState: Record<string, boolean>;
  selectedState: string[];
  metadataByValue: Map<string, SidebarTreeNodeMeta>;
  nodeForValue: (value: string | undefined) => SidebarNavigatorNode | null;
  findNodeByLabel: (label: string) => SidebarNavigatorNode | null;
  activationIntentForValue: (value: string | undefined) => SidebarNavigatorIntent | null;
  expansionIntentForValue: (
    value: string | undefined,
    expanded: boolean,
  ) => SidebarNavigatorIntent | null;
  toggleSectionIntentForValue: (value: string | undefined) => SidebarNavigatorIntent | null;
  keyboardIntentForNode: (
    value: string | undefined,
    key: string,
    shiftKey?: boolean,
  ) => SidebarNavigatorIntent | null;
  contextMenuIntentForValue: (
    value: string | undefined,
    source: SidebarContextMenuSource,
  ) => SidebarNavigatorIntent | null;
  projectCommandIntentForValue: (
    value: string | undefined,
    command: ProjectActionCommand,
  ) => SidebarNavigatorIntent | null;
  projectCommandIntentForProject: (
    projectPath: string,
    command: ProjectActionCommand,
  ) => SidebarNavigatorIntent;
}

const encoded = encodeURIComponent;

export function projectTreeValue(projectPath: string): string {
  return `project:${encoded(projectPath)}`;
}

function sectionTreeValue(projectPath: string, section: SidebarSectionKind): string {
  return `section:${section}:${encoded(projectPath)}`;
}

function taskTreeValue(projectPath: string, taskId: string): string {
  return `task:${encoded(projectPath)}:${encoded(taskId)}`;
}

function eventTreeValue(projectPath: string, eventId: string): string {
  return `event:${encoded(projectPath)}:${encoded(eventId)}`;
}

function emptyTreeValue(projectPath: string, section: SidebarSectionKind): string {
  return `empty:${section}:${encoded(projectPath)}`;
}

export function treeProjectPathFromValue(value: string): string | null {
  return value.startsWith("project:")
    ? decodeURIComponent(value.slice("project:".length))
    : null;
}

export function treeSectionFromValue(value: string): SidebarSectionKind | null {
  if (value.startsWith("section:tasks:")) {
    return "tasks";
  }
  if (value.startsWith("section:events:")) {
    return "events";
  }
  return null;
}

function meta(
  data: Omit<SidebarTreeNodeMeta, "accessibleName"> & { accessibleName?: string },
): SidebarTreeNodeMeta {
  return {
    ...data,
    accessibleName: data.accessibleName ?? data.label,
  };
}

function sidebarProjectTreeRecords(
  records: RememberedProjectRecord[],
  projectState: ProjectOpenState,
  sidebarState: RememberedProjectsSidebarState,
): SidebarProjectTreeRecord[] {
  const activeProject =
    projectState.status === "none" || !projectState.project.projectRoot
      ? null
      : projectState.project;
  const activeProjectPath = activeProject?.projectRoot ?? null;
  const recordsByPath = new Map(records.map((record) => [record.projectPath, record]));

  if (activeProjectPath && !recordsByPath.has(activeProjectPath)) {
    recordsByPath.set(activeProjectPath, {
      projectPath: activeProjectPath,
      expanded: true,
      lastOpenedAt: new Date(0).toISOString(),
      lastSelectedTaskName: null,
    });
  }

  return Array.from(recordsByPath.values()).map((record) => {
    const active = record.projectPath === activeProjectPath;
    const missing = Boolean(sidebarState.scanFailures[record.projectPath]);
    const refreshedContents = sidebarState.projectContents[record.projectPath] ?? null;
    const taskNames = active
      ? activeProject?.tasks.map((task) => task.id) ?? []
      : refreshedContents?.taskNames ?? [];
    const eventNames = active && activeProject
      ? activeProject.events
      : refreshedContents?.eventNames ?? [];
    const diagnosticTaskNames =
      active && activeProject?.workflow
        ? activeProject.workflow.tasks
            .filter(
              (task) =>
                taskDeterminationStatus(task) !== "ready"
                || taskDeterminationMessages(task).length > 0,
            )
            .map((task) => task.id)
        : [];
    const diagnosticEventNames =
      active && activeProject?.workflow
        ? activeProject.events.filter(
            (eventId) => eventPlacementStatus(activeProject.workflow, eventId) !== "ready",
          )
        : [];
    return {
      projectPath: record.projectPath,
      projectName: basenameFromPath(record.projectPath),
      expanded: active,
      active,
      taskNames,
      eventNames,
      diagnosticTaskNames,
      diagnosticEventNames,
      missing,
    };
  });
}

function leafNodes(
  project: SidebarProjectTreeRecord,
  names: string[],
  kind: "task" | "event",
  selected: string | null,
): SidebarNavigatorNode[] {
  const section = kind === "task" ? "tasks" : "events";
  if (names.length === 0) {
    const label = kind === "task" ? "No tasks" : "No events";
    return [
      {
        label,
        value: emptyTreeValue(project.projectPath, section),
        meta: meta({ kind: "empty", label, project }),
        selected: false,
      },
    ];
  }

  return names.map((itemName) => {
    const value =
      kind === "task"
        ? taskTreeValue(project.projectPath, itemName)
        : eventTreeValue(project.projectPath, itemName);
    const diagnostic =
      kind === "task"
        ? project.diagnosticTaskNames.includes(itemName)
        : project.diagnosticEventNames.includes(itemName);
    return {
      label: itemName,
      value,
      meta: meta({ kind, label: itemName, project, itemName, diagnostic }),
      selected: project.active && itemName === selected,
    };
  });
}

function buildNavigatorFromTreeRecords(
  treeRecords: SidebarProjectTreeRecord[],
  selectedTaskId: string | null,
  selectedEventId: string | null,
  sectionExpandedState: Record<string, boolean> = {},
): SidebarNavigatorModel {
  const metadataByValue = new Map<string, SidebarTreeNodeMeta>();
  const nodesByValue = new Map<string, SidebarNavigatorNode>();
  const expandedState: Record<string, boolean> = {};
  const selectedState: string[] = [];
  const remember = (node: SidebarNavigatorNode): SidebarNavigatorNode => {
    metadataByValue.set(node.value, node.meta);
    nodesByValue.set(node.value, node);
    if (node.selected) {
      selectedState.push(node.value);
    }
    node.children?.forEach(remember);
    return node;
  };

  const data = treeRecords.map((project) => {
    const projectValue = projectTreeValue(project.projectPath);
    const tasksValue = sectionTreeValue(project.projectPath, "tasks");
    const eventsValue = sectionTreeValue(project.projectPath, "events");
    expandedState[projectValue] = project.expanded;
    expandedState[tasksValue] = sectionExpandedState[tasksValue] ?? true;
    expandedState[eventsValue] = sectionExpandedState[eventsValue] ?? true;

    const children = project.missing
      ? []
      : [
          {
            label: "Tasks",
            value: tasksValue,
            meta: meta({ kind: "section" as const, label: "Tasks", project, section: "tasks" as const }),
            selected: false,
            expanded: expandedState[tasksValue],
            children: leafNodes(
              project,
              project.taskNames,
              "task",
              selectedEventId ? null : selectedTaskId,
            ),
          },
          {
            label: "Events",
            value: eventsValue,
            meta: meta({ kind: "section" as const, label: "Events", project, section: "events" as const }),
            selected: false,
            expanded: expandedState[eventsValue],
            children: leafNodes(
              project,
              project.eventNames,
              "event",
              selectedTaskId ? null : selectedEventId,
            ),
          },
        ];

    return remember({
      label: project.projectName,
      value: projectValue,
      meta: meta({ kind: "project", label: project.projectName, project }),
      selected: project.active && selectedTaskId === null && selectedEventId === null,
      expanded: project.expanded,
      children,
    });
  });

  const nodeForValue = (value: string | undefined): SidebarNavigatorNode | null =>
    value ? nodesByValue.get(value) ?? null : null;
  const metaForValue = (value: string | undefined): SidebarTreeNodeMeta | null =>
    nodeForValue(value)?.meta ?? null;

  const activationIntentForValue = (value: string | undefined): SidebarNavigatorIntent | null => {
    const nodeMeta = metaForValue(value);
    if (!nodeMeta || nodeMeta.kind === "section" || nodeMeta.kind === "empty") {
      return null;
    }
    if (nodeMeta.kind === "project") {
      return { kind: "activate-project", projectPath: nodeMeta.project.projectPath };
    }
    if (nodeMeta.kind === "task" && nodeMeta.itemName) {
      return {
        kind: "select-task",
        projectPath: nodeMeta.project.projectPath,
        taskId: nodeMeta.itemName,
      };
    }
    if (nodeMeta.kind === "event" && nodeMeta.itemName) {
      return {
        kind: "select-event",
        projectPath: nodeMeta.project.projectPath,
        eventId: nodeMeta.itemName,
      };
    }
    return null;
  };

  const expansionIntentForValue = (
    value: string | undefined,
    expanded: boolean,
  ): SidebarNavigatorIntent | null => {
    const nodeMeta = metaForValue(value);
    if (!value || !nodeMeta) {
      return null;
    }
    if (nodeMeta.kind === "section") {
      return { kind: "set-section-expanded", value, expanded };
    }
    if (nodeMeta.kind === "project") {
      return { kind: "toggle-project-expansion", projectPath: nodeMeta.project.projectPath };
    }
    return null;
  };

  const toggleSectionIntentForValue = (value: string | undefined): SidebarNavigatorIntent | null => {
    const nodeMeta = metaForValue(value);
    if (!value || nodeMeta?.kind !== "section") {
      return null;
    }
    return {
      kind: "set-section-expanded",
      value,
      expanded: !(expandedState[value] ?? true),
    };
  };

  const contextMenuIntentForValue = (
    value: string | undefined,
    source: SidebarContextMenuSource,
  ): SidebarNavigatorIntent | null => {
    const nodeMeta = metaForValue(value);
    if (!nodeMeta) {
      return null;
    }
    if (nodeMeta.kind === "project") {
      return { kind: "open-project-context-menu", projectPath: nodeMeta.project.projectPath, source };
    }
    if (nodeMeta.kind === "task" && nodeMeta.itemName) {
      return {
        kind: "open-task-context-menu",
        projectPath: nodeMeta.project.projectPath,
        taskId: nodeMeta.itemName,
        source,
      };
    }
    if (nodeMeta.kind === "event" && nodeMeta.itemName) {
      return {
        kind: "open-event-context-menu",
        projectPath: nodeMeta.project.projectPath,
        eventId: nodeMeta.itemName,
        source,
      };
    }
    return null;
  };

  const keyboardIntentForNode = (
    value: string | undefined,
    key: string,
    shiftKey = false,
  ): SidebarNavigatorIntent | null => {
    const nodeMeta = metaForValue(value);
    if (!nodeMeta) {
      return null;
    }
    if (key === "ArrowLeft" || key === "ArrowRight") {
      if (nodeMeta.kind === "section" && value) {
        const currentlyExpanded = expandedState[value] ?? true;
        const shouldToggle =
          (key === "ArrowLeft" && currentlyExpanded) ||
          (key === "ArrowRight" && !currentlyExpanded);
        return shouldToggle ? { kind: "set-section-expanded", value, expanded: !currentlyExpanded } : null;
      }
      if (nodeMeta.kind === "project") {
        const shouldToggle =
          (key === "ArrowLeft" && nodeMeta.project.expanded) ||
          (key === "ArrowRight" && !nodeMeta.project.expanded);
        return shouldToggle
          ? { kind: "toggle-project-expansion", projectPath: nodeMeta.project.projectPath }
          : null;
      }
      return null;
    }
    if (key === "Enter" || key === " ") {
      return nodeMeta.kind === "section"
        ? toggleSectionIntentForValue(value)
        : activationIntentForValue(value);
    }
    if (key === "ContextMenu" || (key === "F10" && shiftKey)) {
      return contextMenuIntentForValue(value, "keyboard");
    }
    return null;
  };

  return {
    nodes: data,
    data,
    treeRecords,
    expandedState,
    selectedState,
    metadataByValue,
    nodeForValue,
    findNodeByLabel: (label) => data.flatMap(flattenNodes).find((node) => node.label === label) ?? null,
    activationIntentForValue,
    expansionIntentForValue,
    toggleSectionIntentForValue,
    keyboardIntentForNode,
    contextMenuIntentForValue,
    projectCommandIntentForValue: (value, command) => {
      const nodeMeta = metaForValue(value);
      return nodeMeta?.kind === "project"
        ? { kind: "run-project-command", projectPath: nodeMeta.project.projectPath, command }
        : null;
    },
    projectCommandIntentForProject: (projectPath, command) => ({
      kind: "run-project-command",
      projectPath,
      command,
    }),
  };
}

function flattenNodes(node: SidebarNavigatorNode): SidebarNavigatorNode[] {
  return [node, ...(node.children ?? []).flatMap(flattenNodes)];
}

export function buildSidebarNavigatorModel(input: SidebarNavigatorInput): SidebarNavigatorModel {
  return buildNavigatorFromTreeRecords(
    sidebarProjectTreeRecords(input.records, input.projectState, input.sidebarState),
    input.selectedTaskId,
    input.selectedEventId,
    input.sectionExpandedState ?? {},
  );
}

export function buildSidebarTreeModel(
  treeRecords: SidebarProjectTreeRecord[],
  selectedTaskId: string | null,
  selectedEventId: string | null,
  sectionExpandedState: Record<string, boolean> = {},
): SidebarNavigatorModel {
  return buildNavigatorFromTreeRecords(
    treeRecords,
    selectedTaskId,
    selectedEventId,
    sectionExpandedState,
  );
}
