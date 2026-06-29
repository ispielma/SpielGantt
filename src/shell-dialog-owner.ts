import {
  emptyDeleteEventDialogState,
  emptyEventDialogState,
  emptyNewProjectDialogState,
  emptyRenameEventDialogState,
  emptyRenameTaskDialogState,
  emptyTaskDialogState,
  type DeleteEventDialogState,
  type EventDialogState,
  type NewProjectDialogState,
  type RenameEventDialogState,
  type RenameTaskDialogState,
  type TaskDialogState,
} from "./shell-dialogs.tsx";
import {
  deleteProjectConfirmationPhrase,
  emptyDeleteProjectDialogState,
  type DeleteProjectDialogState,
} from "./delete-project-dialog.tsx";
import { emptyDeleteTaskDialogState, type DeleteTaskDialogState } from "./delete-task-dialog.tsx";

interface DialogOwnerExternalClosers {
  closeTaskFromFolderDialog: () => void;
  closeAlignmentDialog: () => void;
}

interface DialogOwnerSnapshot {
  newProjectDialogState: NewProjectDialogState;
  taskDialogState: TaskDialogState;
  renameTaskDialogState: RenameTaskDialogState;
  eventDialogState: EventDialogState;
  renameEventDialogState: RenameEventDialogState;
  deleteEventDialogState: DeleteEventDialogState;
  deleteProjectDialogState: DeleteProjectDialogState;
  deleteTaskDialogState: DeleteTaskDialogState;
}

export type ShellDialogOwner = ReturnType<typeof createShellDialogOwner>;

export function createShellDialogOwner(externalClosers: DialogOwnerExternalClosers) {
  let newProjectDialogState: NewProjectDialogState = { ...emptyNewProjectDialogState };
  let taskDialogState: TaskDialogState = { ...emptyTaskDialogState };
  let renameTaskDialogState: RenameTaskDialogState = { ...emptyRenameTaskDialogState };
  let eventDialogState: EventDialogState = { ...emptyEventDialogState };
  let renameEventDialogState: RenameEventDialogState = { ...emptyRenameEventDialogState };
  let deleteEventDialogState: DeleteEventDialogState = { ...emptyDeleteEventDialogState };
  let deleteProjectDialogState: DeleteProjectDialogState = {
    ...emptyDeleteProjectDialogState,
  };
  let deleteTaskDialogState: DeleteTaskDialogState = { ...emptyDeleteTaskDialogState };

  const closeNewProjectDialog = () => {
    newProjectDialogState = { ...emptyNewProjectDialogState };
  };
  const closeTaskDialog = () => {
    taskDialogState = { ...emptyTaskDialogState };
  };
  const closeRenameTaskDialog = () => {
    renameTaskDialogState = { ...emptyRenameTaskDialogState };
  };
  const closeEventDialog = () => {
    eventDialogState = { ...emptyEventDialogState };
  };
  const closeRenameEventDialog = () => {
    renameEventDialogState = { ...emptyRenameEventDialogState };
  };
  const closeDeleteEventDialog = () => {
    deleteEventDialogState = { ...emptyDeleteEventDialogState };
  };
  const closeDeleteProjectDialog = () => {
    deleteProjectDialogState = { ...emptyDeleteProjectDialogState };
  };
  const closeDeleteTaskDialog = () => {
    deleteTaskDialogState = { ...emptyDeleteTaskDialogState };
  };

  const closeOwnedDialogs = () => {
    closeNewProjectDialog();
    closeTaskDialog();
    closeRenameTaskDialog();
    closeEventDialog();
    closeRenameEventDialog();
    closeDeleteEventDialog();
    closeDeleteProjectDialog();
    closeDeleteTaskDialog();
  };

  const closeOverlayDialogs = () => {
    closeOwnedDialogs();
    externalClosers.closeTaskFromFolderDialog();
    externalClosers.closeAlignmentDialog();
  };

  const closeDialogsForProjectsMenuOpen = () => {
    closeRenameTaskDialog();
    closeRenameEventDialog();
    closeDeleteEventDialog();
    closeDeleteTaskDialog();
  };

  const closeDialogsForTaskContextMenu = () => {
    closeRenameTaskDialog();
    closeRenameEventDialog();
    closeDeleteEventDialog();
    closeDeleteProjectDialog();
    closeDeleteTaskDialog();
  };

  const closeDialogsForEventContextMenu = () => {
    closeRenameTaskDialog();
    closeRenameEventDialog();
    closeDeleteEventDialog();
    closeDeleteTaskDialog();
  };

  const openNewProjectDialog = () => {
    closeOverlayDialogs();
    newProjectDialogState = {
      open: true,
      projectName: "",
      parentDestination: null,
      errorMessage: null,
      choosingParentDestination: false,
      submitting: false,
    };
  };

  const openTaskDialog = (
    projectPath: string,
    mode: NonNullable<TaskDialogState["mode"]> = "create",
    selectedTaskId = "",
  ) => {
    closeOverlayDialogs();
    taskDialogState = {
      open: true,
      projectPath,
      mode,
      selectedTaskId,
      taskName: "",
      errorMessage: null,
      submitting: false,
    };
  };

  const openCreateEventDialog = (projectPath: string, assignToTaskId?: string) => {
    closeOverlayDialogs();
    eventDialogState = {
      open: true,
      projectPath,
      assignToTaskId,
      eventName: "",
      errorMessage: null,
      submitting: false,
    };
  };

  const openRenameTaskDialog = (projectPath: string, taskId: string) => {
    closeOverlayDialogs();
    renameTaskDialogState = {
      open: true,
      projectPath,
      taskId,
      taskName: taskId,
      errorMessage: null,
      submitting: false,
    };
  };

  const openRenameEventDialog = (
    projectPath: string,
    eventId: string,
    referencedTaskIds: string[],
  ) => {
    closeOverlayDialogs();
    renameEventDialogState = {
      open: true,
      projectPath,
      eventId,
      eventName: eventId,
      referencedTaskIds,
      errorMessage: null,
      submitting: false,
    };
  };

  const openDeleteEventDialog = (
    projectPath: string,
    eventId: string,
    referencedTaskIds: string[],
  ) => {
    closeOverlayDialogs();
    deleteEventDialogState = {
      open: true,
      projectPath,
      eventId,
      eventName: eventId,
      referencedTaskIds,
      errorMessage: null,
      submitting: false,
    };
  };

  const openDeleteTaskDialog = (projectPath: string, taskId: string) => {
    closeOverlayDialogs();
    deleteTaskDialogState = {
      open: true,
      projectPath,
      taskId,
      taskName: taskId,
      errorMessage: null,
      submitting: null,
    };
  };

  const openDeleteProjectDialog = (projectPath: string, projectName: string) => {
    closeOverlayDialogs();
    deleteProjectDialogState = {
      open: true,
      projectPath,
      projectName,
      confirmationText: "",
      confirmationPhrase: deleteProjectConfirmationPhrase(projectName),
      errorMessage: null,
      submitting: false,
    };
  };

  return {
    get snapshot(): DialogOwnerSnapshot {
      return {
        newProjectDialogState,
        taskDialogState,
        renameTaskDialogState,
        eventDialogState,
        renameEventDialogState,
        deleteEventDialogState,
        deleteProjectDialogState,
        deleteTaskDialogState,
      };
    },
    getNewProjectDialogState: () => newProjectDialogState,
    setNewProjectDialogState: (state: NewProjectDialogState) => {
      newProjectDialogState = state;
    },
    getDeleteProjectDialogState: () => deleteProjectDialogState,
    setDeleteProjectDialogState: (state: DeleteProjectDialogState) => {
      deleteProjectDialogState = state;
    },
    getTaskDialogState: () => taskDialogState,
    setTaskDialogState: (state: TaskDialogState) => {
      taskDialogState = state;
    },
    getRenameTaskDialogState: () => renameTaskDialogState,
    setRenameTaskDialogState: (state: RenameTaskDialogState) => {
      renameTaskDialogState = state;
    },
    getEventDialogState: () => eventDialogState,
    setEventDialogState: (state: EventDialogState) => {
      eventDialogState = state;
    },
    getRenameEventDialogState: () => renameEventDialogState,
    setRenameEventDialogState: (state: RenameEventDialogState) => {
      renameEventDialogState = state;
    },
    getDeleteEventDialogState: () => deleteEventDialogState,
    setDeleteEventDialogState: (state: DeleteEventDialogState) => {
      deleteEventDialogState = state;
    },
    getDeleteTaskDialogState: () => deleteTaskDialogState,
    setDeleteTaskDialogState: (state: DeleteTaskDialogState) => {
      deleteTaskDialogState = state;
    },
    closeNewProjectDialog,
    closeTaskDialog,
    closeRenameTaskDialog,
    closeEventDialog,
    closeRenameEventDialog,
    closeDeleteEventDialog,
    closeDeleteProjectDialog,
    closeDeleteTaskDialog,
    closeAlignmentDialog: externalClosers.closeAlignmentDialog,
    closeOverlayDialogs,
    closeDialogsForProjectsMenuOpen,
    closeDialogsForTaskContextMenu,
    closeDialogsForEventContextMenu,
    openNewProjectDialog,
    openTaskDialog,
    openCreateEventDialog,
    openRenameTaskDialog,
    openRenameEventDialog,
    openDeleteEventDialog,
    openDeleteTaskDialog,
    openDeleteProjectDialog,
  };
}
