import type { DeleteTaskDialogState } from "./delete-task-dialog.tsx";
import {
  type RenameTaskDialogState,
  type TaskDialogState,
} from "./shell-dialogs.tsx";
import type {
  OpenProjectResult,
  TaskCreator,
  TaskDeleter,
  TaskRelativeInserter,
  TaskRenamer,
  TaskDeleteMode,
} from "./shell-types.ts";
import { createTaskForDialog } from "./task-create-workflows.ts";
import type { TreeSelectionState } from "./shell-project-session.ts";
import {
  refreshProjectAfterMutation,
  runDialogMutation,
  showMutationValidationFailure,
} from "./mutation-workflow.ts";

interface WorkflowEffects {
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

interface TaskRefreshDeps {
  currentProjectRoot: () => string | null;
  refreshProjectState: (
    project: OpenProjectResult,
    preferredTaskId?: string | null,
    expandRememberedRecord?: boolean | null,
    preferredEventId?: string | null,
  ) => void;
  refreshRememberedProjectTasks: (projectPath: string) => Promise<void>;
}

export interface TaskCreationWorkflowDeps extends WorkflowEffects, TaskRefreshDeps {
  getState: () => TaskDialogState;
  setState: (state: TaskDialogState) => void;
  close: () => void;
  closeTaskAndEventMenus: () => void;
  createTaskAction: TaskCreator;
  insertTaskRelativeAction: TaskRelativeInserter;
}

export interface TaskCreationWorkflow {
  setName: (taskName: string) => void;
  cancel: () => void;
  submit: (taskName: string) => Promise<void>;
}

export function createTaskCreationWorkflow(
  deps: TaskCreationWorkflowDeps,
): TaskCreationWorkflow {
  return {
    setName: (taskName) => {
      deps.setState({
        ...deps.getState(),
        taskName,
        errorMessage: null,
      });
      deps.render();
    },
    cancel: () => {
      deps.close();
      deps.closeTaskAndEventMenus();
      deps.render();
    },
    submit: async (taskName) => {
      const dialogState = deps.getState();
      const projectRoot = dialogState.projectPath ?? deps.currentProjectRoot();
      if (!projectRoot || !taskName) {
        showMutationValidationFailure({
          getState: deps.getState,
          setState: deps.setState,
          render: deps.render,
          message: "Enter a task name before creating a task.",
        });
        return;
      }
      await runDialogMutation({
        ...deps,
        getState: deps.getState,
        setState: deps.setState,
        close: deps.close,
        submitting: true,
        idleSubmitting: false,
        prepareState: (state) => ({ ...state, taskName }),
        invoke: () => createTaskForDialog(
          projectRoot,
          dialogState.mode,
          dialogState.selectedTaskId,
          taskName,
          {
            createTaskAction: deps.createTaskAction,
            insertTaskRelativeAction: deps.insertTaskRelativeAction,
          },
        ),
        onSuccess: async (result) => {
          await refreshProjectAfterMutation({
            projectPath: projectRoot,
            project: result.project,
            currentProjectRoot: deps.currentProjectRoot,
            refreshRememberedProjectTasks: deps.refreshRememberedProjectTasks,
            treatReturnedProjectRootAsActive: true,
            refreshProjectState: (project) => {
              deps.refreshProjectState(project, taskName, null, null);
            },
          });
        },
      });
    },
  };
}

export interface TaskRenameWorkflowDeps extends WorkflowEffects, TaskRefreshDeps {
  getState: () => RenameTaskDialogState;
  setState: (state: RenameTaskDialogState) => void;
  close: () => void;
  closeTaskAndEventMenus: () => void;
  renameTaskAction: TaskRenamer;
}

export interface TaskRenameWorkflow {
  setName: (taskName: string) => void;
  cancel: () => void;
  submit: (taskName: string) => Promise<void>;
}

export function createTaskRenameWorkflow(
  deps: TaskRenameWorkflowDeps,
): TaskRenameWorkflow {
  return {
    setName: (taskName) => {
      deps.setState({
        ...deps.getState(),
        taskName,
        errorMessage: null,
      });
      deps.render();
    },
    cancel: () => {
      deps.close();
      deps.closeTaskAndEventMenus();
      deps.render();
    },
    submit: async (taskName) => {
      const dialogState = deps.getState();
      const projectPath = dialogState.projectPath;
      const taskId = dialogState.taskId;
      if (!projectPath || !taskId || !taskName) {
        showMutationValidationFailure({
          getState: deps.getState,
          setState: deps.setState,
          render: deps.render,
          message: "Enter a task name before renaming the task.",
        });
        return;
      }
      await runDialogMutation({
        ...deps,
        getState: deps.getState,
        setState: deps.setState,
        close: deps.close,
        submitting: true,
        idleSubmitting: false,
        prepareState: (state) => ({ ...state, taskName }),
        invoke: () => deps.renameTaskAction(projectPath, taskId, taskName),
        onSuccess: async (result) => {
          await refreshProjectAfterMutation({
            projectPath,
            project: result.project,
            currentProjectRoot: deps.currentProjectRoot,
            refreshRememberedProjectTasks: deps.refreshRememberedProjectTasks,
            refreshProjectState: (project) => {
              deps.refreshProjectState(project, taskName);
            },
          });
        },
      });
    },
  };
}

export interface TaskDeleteWorkflowDeps extends WorkflowEffects, TaskRefreshDeps {
  getState: () => DeleteTaskDialogState;
  setState: (state: DeleteTaskDialogState) => void;
  close: () => void;
  getTreeSelectionState: () => TreeSelectionState;
  updateTreeSelectionState: (patch: Partial<TreeSelectionState>) => void;
  deleteTaskAction: TaskDeleter;
}

export interface TaskDeleteWorkflow {
  cancel: () => void;
  submit: (mode: TaskDeleteMode) => Promise<void>;
}

export function createTaskDeleteWorkflow(
  deps: TaskDeleteWorkflowDeps,
): TaskDeleteWorkflow {
  return {
    cancel: () => {
      deps.close();
      deps.render();
    },
    submit: async (mode) => {
      const dialogState = deps.getState();
      if (!dialogState.open) {
        deps.render();
        return;
      }
      await runDialogMutation({
        ...deps,
        getState: deps.getState,
        setState: deps.setState,
        close: deps.close,
        submitting: mode,
        idleSubmitting: null,
        invoke: () => deps.deleteTaskAction(
          dialogState.projectPath,
          dialogState.taskId,
          mode,
        ),
        onSuccess: async (result) => {
          await refreshProjectAfterMutation({
            projectPath: dialogState.projectPath,
            project: result.project,
            currentProjectRoot: deps.currentProjectRoot,
            refreshRememberedProjectTasks: deps.refreshRememberedProjectTasks,
            refreshProjectState: (project) => {
              deps.refreshProjectState(
                project,
                deps.getTreeSelectionState().selectedTaskId,
              );
              if (deps.getTreeSelectionState().selectedTaskId === dialogState.taskId) {
                deps.updateTreeSelectionState({ selectedTaskId: null });
              }
            },
          });
        },
      });
    },
  };
}
