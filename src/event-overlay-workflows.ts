import {
  type DeleteEventDialogState,
  type EventDialogState,
  type RenameEventDialogState,
} from "./shell-dialogs.tsx";
import type { TreeSelectionState } from "./shell-project-session.ts";
import type {
  EventCreator,
  EventDeleter,
  EventRenamer,
  OpenProjectResult,
  TaskEndsAtSetter,
} from "./shell-types.ts";
import {
  mutationErrorMessage,
  refreshProjectAfterMutation,
  runDialogMutation,
  showMutationValidationFailure,
} from "./mutation-workflow.ts";

interface WorkflowEffects {
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

interface EventRefreshDeps {
  currentProjectRoot: () => string | null;
  refreshProjectState: (
    project: OpenProjectResult,
    preferredTaskId?: string | null,
    expandRememberedRecord?: boolean | null,
    preferredEventId?: string | null,
  ) => void;
  refreshRememberedProjectTasks: (projectPath: string) => Promise<void>;
}

export interface EventCreationWorkflowDeps extends WorkflowEffects {
  getState: () => EventDialogState;
  setState: (state: EventDialogState) => void;
  close: () => void;
  createEventAction: EventCreator;
  setTaskEndsAtAction: TaskEndsAtSetter;
  refreshProjectState: EventRefreshDeps["refreshProjectState"];
}

export interface EventCreationWorkflow {
  setName: (eventName: string) => void;
  cancel: () => void;
  submit: (eventName: string) => Promise<void>;
}

export function createEventCreationWorkflow(
  deps: EventCreationWorkflowDeps,
): EventCreationWorkflow {
  return {
    setName: (eventName) => {
      deps.setState({
        ...deps.getState(),
        eventName,
        errorMessage: null,
      });
      deps.render();
    },
    cancel: () => {
      deps.close();
      deps.render();
    },
    submit: async (eventName) => {
      const dialogState = deps.getState();
      const projectRoot = dialogState.projectPath;
      if (!projectRoot || !eventName) {
        showMutationValidationFailure({
          getState: deps.getState,
          setState: deps.setState,
          render: deps.render,
          message: "Enter an event name before creating an event.",
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
        prepareState: (state) => ({ ...state, eventName }),
        invoke: () => deps.createEventAction(projectRoot, eventName),
        onSuccess: async (result) => {
          if (!dialogState.assignToTaskId) {
            deps.refreshProjectState(result.project, null, null, eventName);
            return;
          }

          try {
            const assignment = await deps.setTaskEndsAtAction(
              projectRoot,
              dialogState.assignToTaskId,
              eventName,
              false,
            );
            deps.refreshProjectState(
              assignment.project,
              dialogState.assignToTaskId,
              null,
              null,
            );
          } catch (error) {
            const message = mutationErrorMessage(error);
            deps.refreshProjectState(result.project, dialogState.assignToTaskId, null, null);
            deps.setState({
              ...deps.getState(),
              submitting: false,
              errorMessage:
                `Created event '${eventName}', but could not set it as the task end event: ` +
                `${message} Close this dialog and select the event manually, or try another event.`,
            });
            deps.setActionError(error);
            return "keep-open";
          }
        },
      });
    },
  };
}

export interface EventRenameWorkflowDeps extends WorkflowEffects, EventRefreshDeps {
  getState: () => RenameEventDialogState;
  setState: (state: RenameEventDialogState) => void;
  close: () => void;
  renameEventAction: EventRenamer;
}

export interface EventRenameWorkflow {
  setName: (eventName: string) => void;
  cancel: () => void;
  submit: (eventName: string) => Promise<void>;
}

export function createEventRenameWorkflow(
  deps: EventRenameWorkflowDeps,
): EventRenameWorkflow {
  return {
    setName: (eventName) => {
      deps.setState({
        ...deps.getState(),
        eventName,
        errorMessage: null,
      });
      deps.render();
    },
    cancel: () => {
      deps.close();
      deps.render();
    },
    submit: async (eventName) => {
      const dialogState = deps.getState();
      const { projectPath, eventId } = dialogState;
      if (!projectPath || !eventId || !eventName) {
        showMutationValidationFailure({
          getState: deps.getState,
          setState: deps.setState,
          render: deps.render,
          message: "Enter an event name before renaming the event.",
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
        prepareState: (state) => ({ ...state, eventName }),
        invoke: () => deps.renameEventAction(projectPath, eventId, eventName),
        onSuccess: async (result) => {
          await refreshProjectAfterMutation({
            projectPath,
            project: result.project,
            currentProjectRoot: deps.currentProjectRoot,
            refreshRememberedProjectTasks: deps.refreshRememberedProjectTasks,
            refreshProjectState: (project) => {
              deps.refreshProjectState(project, null, null, eventName);
            },
          });
        },
      });
    },
  };
}

export interface EventDeleteWorkflowDeps extends WorkflowEffects, EventRefreshDeps {
  getState: () => DeleteEventDialogState;
  setState: (state: DeleteEventDialogState) => void;
  close: () => void;
  getTreeSelectionState: () => TreeSelectionState;
  updateTreeSelectionState: (patch: Partial<TreeSelectionState>) => void;
  deleteEventAction: EventDeleter;
}

export interface EventDeleteWorkflow {
  cancel: () => void;
  submit: () => Promise<void>;
}

export function createEventDeleteWorkflow(
  deps: EventDeleteWorkflowDeps,
): EventDeleteWorkflow {
  return {
    cancel: () => {
      deps.close();
      deps.render();
    },
    submit: async () => {
      const dialogState = deps.getState();
      if (!dialogState.open || dialogState.referencedTaskIds.length > 0) {
        deps.render();
        return;
      }
      await runDialogMutation({
        ...deps,
        getState: deps.getState,
        setState: deps.setState,
        close: deps.close,
        submitting: true,
        idleSubmitting: false,
        invoke: () => deps.deleteEventAction(
          dialogState.projectPath,
          dialogState.eventId,
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
              if (deps.getTreeSelectionState().selectedEventId === dialogState.eventId) {
                deps.updateTreeSelectionState({ selectedEventId: null });
              }
            },
          });
        },
      });
    },
  };
}
