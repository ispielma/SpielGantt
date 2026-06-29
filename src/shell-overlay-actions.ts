import {
  type NewProjectDialogState,
} from "./shell-dialogs.tsx";
import type { DeleteProjectDialogState } from "./delete-project-dialog.tsx";
import type {
  EventCreationWorkflow,
  EventDeleteWorkflow,
  EventRenameWorkflow,
} from "./event-overlay-workflows.ts";
import type { OverlayAction } from "./shell-overlays.tsx";
import { resolveSelectedTaskId } from "./shell-project-session.ts";
import type { OpenProjectResult, ProjectCreator } from "./shell-types.ts";
import type { TaskFromFolderWorkflowController } from "./task-from-folder-workflow.ts";
import { runDialogMutation, showMutationValidationFailure } from "./mutation-workflow.ts";
import type {
  TaskCreationWorkflow,
  TaskDeleteWorkflow,
  TaskRenameWorkflow,
} from "./task-overlay-workflows.ts";

interface ShellOverlayActionContext {
  getNewProjectDialogState: () => NewProjectDialogState;
  setNewProjectDialogState: (state: NewProjectDialogState) => void;
  getDeleteProjectDialogState: () => DeleteProjectDialogState;
  setDeleteProjectDialogState: (state: DeleteProjectDialogState) => void;
  getTaskFromFolderWorkflow: () => TaskFromFolderWorkflowController | null;
  closeNewProjectDialog: () => void;
  closeDeleteProjectDialog: () => void;
  closeAlignmentDialog: () => void;
  pickProjectDestination: (defaultPath?: string | null) => Promise<string | null>;
  confirmAlignment: () => Promise<void>;
  confirmDeleteProject: () => Promise<void>;
  setProjectsMenuOpen: (open: boolean) => void;
  currentProjectRoot: () => string | null;
  refreshProjectState: (
    project: OpenProjectResult,
    preferredTaskId?: string | null,
    expandRememberedRecord?: boolean | null,
    preferredEventId?: string | null,
  ) => void;
  startProjectWatch: (projectRoot: string | null) => Promise<void>;
  updateWindowTitleForProject: (projectRoot: string | null) => Promise<void>;
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  createProjectAction: ProjectCreator;
  taskCreationWorkflow: TaskCreationWorkflow;
  taskRenameWorkflow: TaskRenameWorkflow;
  taskDeleteWorkflow: TaskDeleteWorkflow;
  eventCreationWorkflow: EventCreationWorkflow;
  eventRenameWorkflow: EventRenameWorkflow;
  eventDeleteWorkflow: EventDeleteWorkflow;
  render: () => void;
}

export function createShellOverlayActionHandler(context: ShellOverlayActionContext) {
  return async (action: OverlayAction) => {
    switch (action.kind) {
      case "new-project-name":
        context.setNewProjectDialogState({
          ...context.getNewProjectDialogState(),
          projectName: action.value,
          errorMessage: null,
        });
        break;
      case "choose-new-project-parent": {
        const dialogState = context.getNewProjectDialogState();
        context.setNewProjectDialogState({
          ...dialogState,
          errorMessage: null,
          choosingParentDestination: true,
        });
        context.render();
        try {
          const selectedDestination = await context.pickProjectDestination(
            dialogState.parentDestination,
          );
          const latestDialogState = context.getNewProjectDialogState();
          if (!latestDialogState.open) {
            break;
          }
          context.setNewProjectDialogState({
            ...latestDialogState,
            parentDestination: selectedDestination ?? latestDialogState.parentDestination,
            choosingParentDestination: false,
          });
        } catch (error) {
          const latestDialogState = context.getNewProjectDialogState();
          context.setNewProjectDialogState({
            ...latestDialogState,
            choosingParentDestination: false,
            errorMessage: error instanceof Error ? error.message : String(error),
          });
          context.setActionError(error);
        }
        break;
      }
      case "create-task-name":
        context.taskCreationWorkflow.setName(action.value);
        return;
      case "rename-task-name":
        context.taskRenameWorkflow.setName(action.value);
        return;
      case "create-event-name":
        context.eventCreationWorkflow.setName(action.value);
        return;
      case "rename-event-name":
        context.eventRenameWorkflow.setName(action.value);
        return;
      case "delete-project-confirmation":
        context.setDeleteProjectDialogState({
          ...context.getDeleteProjectDialogState(),
          confirmationText: action.value,
          errorMessage: null,
        });
        break;
      case "cancel-new-project":
        context.closeNewProjectDialog();
        break;
      case "cancel-new-task":
        context.taskCreationWorkflow.cancel();
        return;
      case "cancel-rename-task":
        context.taskRenameWorkflow.cancel();
        return;
      case "cancel-new-event":
        context.eventCreationWorkflow.cancel();
        return;
      case "cancel-rename-event":
        context.eventRenameWorkflow.cancel();
        return;
      case "cancel-delete-event":
        context.eventDeleteWorkflow.cancel();
        return;
      case "cancel-delete-project":
        context.closeDeleteProjectDialog();
        break;
      case "cancel-delete-task":
        context.taskDeleteWorkflow.cancel();
        return;
      case "select-task-from-folder":
        context.getTaskFromFolderWorkflow()?.select(action.folderPath);
        break;
      case "cancel-task-from-folder":
        context.getTaskFromFolderWorkflow()?.close();
        break;
      case "cancel-alignment":
        context.closeAlignmentDialog();
        break;
      case "apply-alignment":
        await context.confirmAlignment();
        return;
      case "submit-delete-project":
        await context.confirmDeleteProject();
        return;
      case "submit-new-project": {
        const dialogState = context.getNewProjectDialogState();
        if (!action.projectName) {
          showMutationValidationFailure({
            getState: context.getNewProjectDialogState,
            setState: context.setNewProjectDialogState,
            render: context.render,
            message: "Enter a project name before creating a project.",
          });
          break;
        }
        if (!dialogState.parentDestination) {
          showMutationValidationFailure({
            getState: context.getNewProjectDialogState,
            setState: context.setNewProjectDialogState,
            render: context.render,
            message: "Choose a parent destination before creating a project.",
            prepareState: (state) => ({ ...state, projectName: action.projectName }),
          });
          break;
        }
        const parentDestination = dialogState.parentDestination;
        await runDialogMutation({
          getState: context.getNewProjectDialogState,
          setState: context.setNewProjectDialogState,
          close: context.closeNewProjectDialog,
          setActionError: context.setActionError,
          setOperationIdle: context.setOperationIdle,
          render: context.render,
          submitting: true,
          idleSubmitting: false,
          prepareState: (state) => ({ ...state, projectName: action.projectName }),
          invoke: () => context.createProjectAction(
            action.projectName,
            parentDestination,
          ),
          onSuccess: (createdProject) => {
            context.refreshProjectState(
              createdProject,
              resolveSelectedTaskId(createdProject, null),
              true,
            );
            context.setProjectsMenuOpen(false);
          },
          afterClose: async () => {
            await context.startProjectWatch(context.currentProjectRoot());
            await context.updateWindowTitleForProject(context.currentProjectRoot());
          },
        });
        break;
      }
      case "submit-new-task": {
        await context.taskCreationWorkflow.submit(action.taskName);
        return;
      }
      case "submit-rename-task": {
        await context.taskRenameWorkflow.submit(action.taskName);
        return;
      }
      case "submit-new-event": {
        await context.eventCreationWorkflow.submit(action.eventName);
        return;
      }
      case "submit-rename-event": {
        await context.eventRenameWorkflow.submit(action.eventName);
        return;
      }
      case "submit-delete-event": {
        await context.eventDeleteWorkflow.submit();
        return;
      }
      case "submit-delete-task": {
        await context.taskDeleteWorkflow.submit(action.mode);
        return;
      }
      case "submit-task-from-folder":
        await context.getTaskFromFolderWorkflow()?.submit();
        return;
    }

    context.render();
  };
}
