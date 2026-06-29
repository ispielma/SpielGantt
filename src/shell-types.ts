import type { RememberedProjectsSettings } from "./remembered-projects.ts";
import type { ProjectTrashResult } from "./project-trash.ts";
import type { TimelineWorkflow } from "./timeline.ts";

export type { TimelineWorkflow } from "./timeline.ts";

export type TaskStatus = "blocked" | "unblocked" | "done";

export const taskStatusOptions: ReadonlyArray<{ value: TaskStatus; label: TaskStatus }> = [
  { value: "blocked", label: "blocked" },
  { value: "unblocked", label: "unblocked" },
  { value: "done", label: "done" },
];

export interface BackendHealth {
  appName: string;
  version: string;
  core: string;
}

export interface OpenProjectResult {
  selectedPath: string;
  projectRoot: string | null;
  valid: boolean;
  issues: string[];
  agentReadiness?: AgentReadinessStatus;
  projectReadmeContent: string;
  projectReadmeVersion: string;
  events: string[];
  eventReferences: Array<{
    id: string;
    referencedTaskIds: string[];
    blockerTaskIds?: string[];
    blockedTaskIds?: string[];
  }>;
  workflow: TimelineWorkflow | null;
  tasks: ProjectTask[];
}

export interface AgentReadinessStatus {
  ready: boolean;
  agentsMdPresent: boolean;
  skillsDirPresent: boolean;
  metadataPresent: boolean;
  recordedCliPath: string | null;
}

export interface ProjectTask {
  id: string;
  path: string;
  projectRelativePath: string;
  dependencies: string[];
  blocks: Array<{
    id: string;
    kind: "task" | "event";
  }>;
  dependencyReferences: Array<{
    id: string;
    kind: "task" | "event";
  }>;
  dependencyTargets: Array<{
    id: string;
    kind: "task" | "event";
  }>;
  endsAt?: string | null;
  status: TaskStatus | null;
  readmeContent: string;
  readmeVersion: string;
}

export interface TaskActionResult {
  project: OpenProjectResult;
}

export interface ProjectActionResult {
  project: OpenProjectResult;
}

export interface AdoptableTaskFolder {
  folderPath: string;
  projectRelativePath: string;
  taskId: string;
}

export interface TaskNormalizationResult {
  project: OpenProjectResult;
  renames: TaskRename[];
  issues: string[];
  applied: boolean;
}

export interface TaskRename {
  id: string;
  from: string;
  to: string;
}

export interface TaskFolderAlignmentResult {
  project: OpenProjectResult;
  renames: TaskRename[];
  issues: string[];
  applied: boolean;
}

export interface TaskFolderAlignmentOperation {
  renameTaskFolder: {
    taskId: string;
    from: string;
    to: string;
  };
}

export interface TaskFolderAlignmentPreflightIssue {
  targetAlreadyExists: string;
}

export interface TaskFolderAlignmentPlan {
  operations: TaskFolderAlignmentOperation[];
  preflightIssues: TaskFolderAlignmentPreflightIssue[];
}

export interface TaskEdit {
  status: TaskStatus | null;
  readmeContent: string;
  expectedReadmeVersion: string;
}

export interface ProjectReadmeEdit {
  readmeContent: string;
  expectedReadmeVersion: string;
}

export interface ProjectWatchStatus {
  watching: boolean;
  message: string;
}

export interface ProjectWatchSubscription {
  status: ProjectWatchStatus;
  unsubscribe: () => void | Promise<void>;
}

export interface ProjectAgentPrepareResult {
  project: OpenProjectResult;
  outcome: "created" | "refreshed" | "unchanged";
  files: Array<{
    path: string;
    status: "created" | "refreshed" | "unchanged";
  }>;
}

export type ProjectOpenState =
  | { status: "none" }
  | { status: "valid"; project: OpenProjectResult }
  | { status: "invalid"; project: OpenProjectResult };

export type OperationState =
  | { status: "idle" }
  | { status: "error"; message: string };

export type TaskDeleteMode = "remove-from-chart" | "delete-directory";

export type ProjectFolderPicker = (defaultPath?: string | null) => Promise<string | null>;
export type ProjectParentDestinationResolver = () => Promise<string | null>;
export type TaskFolderPicker = () => Promise<string | null>;
export type ProjectOpener = (path: string) => Promise<OpenProjectResult>;
export type ProjectOnboarder = (path: string) => Promise<OpenProjectResult>;
export type ProjectCreator = (
  projectName: string,
  parentDestination: string,
) => Promise<OpenProjectResult>;
export type TaskCreator = (projectRoot: string, id: string) => Promise<TaskActionResult>;
export type TaskRelativeInsertMode = "before" | "after";
export type TaskRelativeInserter = (
  projectRoot: string,
  mode: TaskRelativeInsertMode,
  selectedTaskId: string,
  insertedTaskId: string,
) => Promise<TaskActionResult>;
export type EventCreator = (projectRoot: string, id: string) => Promise<TaskActionResult>;
export type TaskAdopter = (
  projectRoot: string,
  folderPath: string,
  id: string,
) => Promise<TaskActionResult>;
export type AdoptableTaskFolderLister = (
  projectRoot: string,
) => Promise<AdoptableTaskFolder[]>;
export type NormalizationPreviewer = (
  projectRoot: string,
) => Promise<TaskNormalizationResult>;
export type NormalizationApplier = (
  projectRoot: string,
) => Promise<TaskNormalizationResult>;
export type TaskFolderAlignmentPreviewer = (
  projectRoot: string,
) => Promise<TaskFolderAlignmentPlan>;
export type TaskFolderAlignmentApplier = (
  projectRoot: string,
  plan: TaskFolderAlignmentPlan,
) => Promise<TaskFolderAlignmentResult>;
export type TaskEditor = (
  projectRoot: string,
  taskId: string,
  edit: TaskEdit,
) => Promise<TaskActionResult>;
export type ProjectReadmeEditor = (
  projectRoot: string,
  edit: ProjectReadmeEdit,
) => Promise<ProjectActionResult>;
export type TaskEndsAtSetter = (
  projectRoot: string,
  taskId: string,
  eventId: string | null,
  clear: boolean,
) => Promise<TaskActionResult>;
export type TaskRenamer = (
  projectRoot: string,
  taskId: string,
  newTaskId: string,
) => Promise<TaskActionResult>;
export type TaskDeleter = (
  projectRoot: string,
  taskId: string,
  mode: TaskDeleteMode,
) => Promise<TaskActionResult>;
export type EventRenamer = (
  projectRoot: string,
  eventId: string,
  newEventId: string,
) => Promise<TaskActionResult>;
export type EventDeleter = (
  projectRoot: string,
  eventId: string,
) => Promise<TaskActionResult>;
export type DependencyAdder = (
  projectRoot: string,
  taskId: string,
  blockerId: string,
) => Promise<TaskActionResult>;
export type DependencyRemover = (
  projectRoot: string,
  taskId: string,
  blockerId: string,
) => Promise<TaskActionResult>;
export type ProjectFolderOpener = (projectRoot: string) => Promise<void>;
export type TaskFolderOpener = (taskFolderPath: string) => Promise<void>;
export type AgentScaffoldingPreparer = (
  projectRoot: string,
) => Promise<ProjectAgentPrepareResult>;
export type ProjectChangeSubscriber = (
  projectRoot: string,
  refreshProject: () => Promise<void>,
) => Promise<ProjectWatchSubscription>;
export type WindowTitleSetter = (title: string) => Promise<void>;
export type ProjectFolderTrasher = (projectRoot: string) => Promise<ProjectTrashResult>;

export interface ProjectLifecycleCapabilities {
  loadHealth: () => Promise<BackendHealth>;
  openSelectedProject: ProjectOpener;
  onboardProjectAction: ProjectOnboarder;
  createProjectAction: ProjectCreator;
  editProjectReadmeAction: ProjectReadmeEditor;
  prepareAgentScaffoldingAction: AgentScaffoldingPreparer;
  previewAlignmentAction: TaskFolderAlignmentPreviewer;
  applyAlignmentAction: TaskFolderAlignmentApplier;
}

export interface ProjectSessionPersistenceCapabilities {
  loadRememberedProjectsSettings:
    | (() => Promise<RememberedProjectsSettings>)
    | (() => RememberedProjectsSettings);
}

export interface TaskMutationCapabilities {
  createTaskAction: TaskCreator;
  insertTaskRelativeAction: TaskRelativeInserter;
  adoptTaskAction: TaskAdopter;
  listAdoptableTaskFoldersAction: AdoptableTaskFolderLister;
  previewNormalization: NormalizationPreviewer;
  applyNormalization: NormalizationApplier;
  editTaskAction: TaskEditor;
  addDependencyAction: DependencyAdder;
  removeDependencyAction: DependencyRemover;
  renameTaskAction: TaskRenamer;
  deleteTaskAction: TaskDeleter;
  setTaskEndsAtAction: TaskEndsAtSetter;
}

export interface EventMutationCapabilities {
  createEventAction: EventCreator;
  renameEventAction: EventRenamer;
  deleteEventAction: EventDeleter;
}

export interface DesktopShellCapabilities {
  pickProjectFolder: ProjectFolderPicker;
  pickProjectParentDestination: ProjectFolderPicker;
  resolveDefaultProjectParent: ProjectParentDestinationResolver;
  pickTaskFolder: TaskFolderPicker;
  openProjectFolderAction: ProjectFolderOpener;
  openTaskFolderAction: TaskFolderOpener;
  setWindowTitle: WindowTitleSetter;
  trashProjectFolderAction: ProjectFolderTrasher;
}

export interface ProjectWatchCapabilities {
  subscribeProjectSessionChanges: ProjectChangeSubscriber;
}

export interface ShellDeps {
  projectLifecycle: ProjectLifecycleCapabilities;
  projectSession: ProjectSessionPersistenceCapabilities;
  taskMutation: TaskMutationCapabilities;
  eventMutation: EventMutationCapabilities;
  desktop: DesktopShellCapabilities;
  projectWatch: ProjectWatchCapabilities;
}

export type ShellDepsInput = {
  [Group in keyof ShellDeps]?: Partial<ShellDeps[Group]>;
};
