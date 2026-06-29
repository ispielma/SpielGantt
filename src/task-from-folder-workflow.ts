import {
  emptyTaskFromFolderDialogState,
  type TaskFromFolderDialogState,
} from "./task-from-folder-dialog.tsx";
import { refreshProjectAfterMutation, runDialogMutation } from "./mutation-workflow.ts";
import type { AdoptableTaskFolderLister, OpenProjectResult, TaskAdopter } from "./shell-types.ts";

interface TaskFromFolderWorkflowDeps {
  listAdoptableTaskFoldersAction: AdoptableTaskFolderLister;
  adoptTaskAction: TaskAdopter;
  currentProjectRoot: () => string | null;
  refreshProjectState: (
    project: OpenProjectResult,
    preferredTaskId: string | null,
    expandRememberedRecord: boolean,
  ) => void;
  refreshRememberedProjectTasks: (projectPath: string) => Promise<void>;
  closeMutationDialogs: () => void;
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

export interface TaskFromFolderWorkflowController {
  getState: () => TaskFromFolderDialogState;
  close: () => void;
  open: (projectPath: string) => Promise<void>;
  select: (folderPath: string) => void;
  submit: () => Promise<void>;
}

export function createTaskFromFolderWorkflow(
  deps: TaskFromFolderWorkflowDeps,
): TaskFromFolderWorkflowController {
  let state: TaskFromFolderDialogState = { ...emptyTaskFromFolderDialogState };

  const close = () => {
    state = { ...emptyTaskFromFolderDialogState };
  };

  return {
    getState: () => state,
    close,
    open: async (projectPath) => {
      deps.closeMutationDialogs();
      deps.render();
      try {
        const candidates = await deps.listAdoptableTaskFoldersAction(projectPath);
        state = {
          open: true,
          projectPath,
          candidates,
          selectedFolderPath: "",
          errorMessage: null,
          submitting: false,
        };
        deps.setOperationIdle();
      } catch (error) {
        deps.setActionError(error);
      }
      deps.render();
    },
    select: (folderPath) => {
      state = { ...state, selectedFolderPath: folderPath, errorMessage: null };
    },
    submit: async () => {
      const candidate = state.candidates.find(
        (folder) => folder.folderPath === state.selectedFolderPath,
      );
      if (!state.open || !candidate) {
        return;
      }

      await runDialogMutation({
        getState: () => state,
        setState: (nextState) => {
          state = nextState;
        },
        close,
        setActionError: deps.setActionError,
        setOperationIdle: deps.setOperationIdle,
        render: deps.render,
        submitting: true,
        idleSubmitting: false,
        invoke: () => deps.adoptTaskAction(
          state.projectPath,
          candidate.folderPath,
          candidate.taskId,
        ),
        onSuccess: async (result) => {
          await refreshProjectAfterMutation({
            projectPath: state.projectPath,
            project: result.project,
            currentProjectRoot: deps.currentProjectRoot,
            refreshRememberedProjectTasks: deps.refreshRememberedProjectTasks,
            treatReturnedProjectRootAsActive: true,
            refreshProjectState: (project) => {
              deps.refreshProjectState(project, candidate.taskId, true);
            },
          });
        },
      });
    },
  };
}
