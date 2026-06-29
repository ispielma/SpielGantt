export type ProjectActionId =
  | "refresh-project"
  | "open-project-folder"
  | "create-task"
  | "create-task-from-folder"
  | "create-event"
  | "find-project"
  | "remove-project"
  | "delete-project";

export type ProjectActionCommand =
  | { kind: "refresh-project" }
  | { kind: "reveal-project-folder" }
  | { kind: "open-create-task-dialog" }
  | { kind: "open-create-task-from-folder-workflow" }
  | { kind: "open-create-event-dialog" }
  | { kind: "find-remembered-project" }
  | { kind: "remove-remembered-project" }
  | { kind: "open-delete-project-dialog" };

export interface ProjectActionContext {
  projectPath: string;
  projectName: string;
  missing: boolean;
  refreshing: boolean;
}

export interface ProjectActionOption {
  kind: "action";
  id: ProjectActionId;
  label: string;
  ariaLabel: string;
  disabled: boolean;
  disabledReason: string | null;
  command: ProjectActionCommand;
}

export interface ProjectActionSeparator {
  kind: "separator";
}

export type ProjectActionEntry = ProjectActionOption | ProjectActionSeparator;

interface ProjectActionDefinition {
  id: ProjectActionId;
  label: string;
  command: ProjectActionCommand;
  ariaLabel: (projectName: string) => string;
  disabledReason?: (context: ProjectActionContext) => string | null;
}

const actionDefinitions: Record<ProjectActionId, ProjectActionDefinition> = {
  "refresh-project": {
    id: "refresh-project",
    label: "Refresh Project",
    command: { kind: "refresh-project" },
    ariaLabel: (projectName) => `Refresh project ${projectName}`,
    disabledReason: (context) => (context.refreshing ? "Project refresh is already running." : null),
  },
  "open-project-folder": {
    id: "open-project-folder",
    label: "Reveal in Finder",
    command: { kind: "reveal-project-folder" },
    ariaLabel: (projectName) => `Reveal project ${projectName} in Finder`,
  },
  "create-task": {
    id: "create-task",
    label: "Create Task",
    command: { kind: "open-create-task-dialog" },
    ariaLabel: (projectName) => `Create task in ${projectName}`,
  },
  "create-task-from-folder": {
    id: "create-task-from-folder",
    label: "Create Task from Folder...",
    command: { kind: "open-create-task-from-folder-workflow" },
    ariaLabel: (projectName) => `Create task from folder in ${projectName}`,
  },
  "create-event": {
    id: "create-event",
    label: "Create Event",
    command: { kind: "open-create-event-dialog" },
    ariaLabel: (projectName) => `Create event in ${projectName}`,
  },
  "find-project": {
    id: "find-project",
    label: "Find Project...",
    command: { kind: "find-remembered-project" },
    ariaLabel: (projectName) => `Find project ${projectName}`,
  },
  "remove-project": {
    id: "remove-project",
    label: "Remove from Sidebar",
    command: { kind: "remove-remembered-project" },
    ariaLabel: (projectName) => `Remove project ${projectName} from sidebar`,
  },
  "delete-project": {
    id: "delete-project",
    label: "Delete Project...",
    command: { kind: "open-delete-project-dialog" },
    ariaLabel: (projectName) => `Delete project ${projectName}`,
  },
};

const availableProjectActionGroups: ProjectActionId[][] = [
  ["refresh-project", "open-project-folder"],
  ["create-task", "create-task-from-folder", "create-event"],
  ["delete-project"],
  ["remove-project"],
];

const missingProjectActionGroups: ProjectActionId[][] = [["find-project", "remove-project"]];

export function projectActionCommandForId(id: ProjectActionId): ProjectActionCommand {
  return actionDefinitions[id].command;
}

function projectActionOptionForId(
  id: ProjectActionId,
  context: ProjectActionContext,
): ProjectActionOption {
  const definition = actionDefinitions[id];
  const disabledReason = definition.disabledReason?.(context) ?? null;
  return {
    kind: "action",
    id: definition.id,
    label: definition.label,
    ariaLabel: definition.ariaLabel(context.projectName),
    disabled: Boolean(disabledReason),
    disabledReason,
    command: definition.command,
  };
}

export function projectActionEntriesForProject(
  context: ProjectActionContext,
): ProjectActionEntry[] {
  const groups = context.missing ? missingProjectActionGroups : availableProjectActionGroups;
  return groups.flatMap((group, index) => {
    const entries = group.map((id) => projectActionOptionForId(id, context));
    return index === 0 ? entries : [{ kind: "separator" as const }, ...entries];
  });
}
