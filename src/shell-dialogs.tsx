import type { TaskFolderAlignmentPlan } from "./shell-types.ts";
import type { TaskCreateMode } from "./task-create-workflows.ts";

export { TaskFolderAlignmentDialog } from "./alignment-dialog.tsx";
export { CreateEventDialog, DeleteEventDialog, RenameEventDialog } from "./event-dialogs.tsx";
export { NewProjectDialog } from "./project-dialogs.tsx";
export { CreateTaskDialog, RenameTaskDialog } from "./task-dialogs.tsx";

export interface NewProjectDialogState {
  open: boolean;
  projectName: string;
  parentDestination: string | null;
  errorMessage: string | null;
  choosingParentDestination: boolean;
  submitting: boolean;
}

export interface TaskFolderAlignmentDialogState {
  open: boolean;
  projectPath: string;
  projectName: string;
  plan: TaskFolderAlignmentPlan | null;
  errorMessage: string | null;
  submitting: boolean;
}

export interface TaskDialogState {
  open: boolean;
  projectPath?: string;
  mode?: TaskCreateMode;
  selectedTaskId?: string;
  taskName: string;
  errorMessage: string | null;
  submitting: boolean;
}

export interface RenameTaskDialogState {
  open: boolean;
  projectPath: string;
  taskId: string;
  taskName: string;
  errorMessage: string | null;
  submitting: boolean;
}

export interface EventDialogState {
  open: boolean;
  projectPath: string;
  assignToTaskId?: string;
  eventName: string;
  errorMessage: string | null;
  submitting: boolean;
}

export interface RenameEventDialogState {
  open: boolean;
  projectPath: string;
  eventId: string;
  eventName: string;
  referencedTaskIds: string[];
  errorMessage: string | null;
  submitting: boolean;
}

export interface DeleteEventDialogState {
  open: boolean;
  projectPath: string;
  eventId: string;
  eventName: string;
  referencedTaskIds: string[];
  errorMessage: string | null;
  submitting: boolean;
}

export const emptyNewProjectDialogState: NewProjectDialogState = {
  open: false,
  projectName: "",
  parentDestination: null,
  errorMessage: null,
  choosingParentDestination: false,
  submitting: false,
};

export const emptyTaskFolderAlignmentDialogState: TaskFolderAlignmentDialogState = {
  open: false,
  projectPath: "",
  projectName: "",
  plan: null,
  errorMessage: null,
  submitting: false,
};

export const emptyTaskDialogState: TaskDialogState = {
  open: false,
  projectPath: "",
  taskName: "",
  errorMessage: null,
  submitting: false,
};

export const emptyRenameTaskDialogState: RenameTaskDialogState = {
  open: false,
  projectPath: "",
  taskId: "",
  taskName: "",
  errorMessage: null,
  submitting: false,
};

export const emptyEventDialogState: EventDialogState = {
  open: false,
  projectPath: "",
  eventName: "",
  errorMessage: null,
  submitting: false,
};

export const emptyRenameEventDialogState: RenameEventDialogState = {
  open: false,
  projectPath: "",
  eventId: "",
  eventName: "",
  referencedTaskIds: [],
  errorMessage: null,
  submitting: false,
};

export const emptyDeleteEventDialogState: DeleteEventDialogState = {
  open: false,
  projectPath: "",
  eventId: "",
  eventName: "",
  referencedTaskIds: [],
  errorMessage: null,
  submitting: false,
};
