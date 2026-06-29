import type {
  DependencyAdder,
  DependencyRemover,
  OpenProjectResult,
  OperationState,
  TaskEdit,
  TaskEditor,
  TaskEndsAtSetter,
} from "./shell-types.ts";
import { mutationErrorMessage } from "./mutation-workflow.ts";

export interface TaskEndsAtControlState {
  errorMessage: string | null;
  submitting: boolean;
  selectedEventId: string;
}

export interface TaskDependencyControlState {
  selectedBlockerId: string;
}

export const emptyTaskEndsAtControlState: TaskEndsAtControlState = {
  errorMessage: null,
  submitting: false,
  selectedEventId: "",
};

export const emptyTaskDependencyControlState: TaskDependencyControlState = {
  selectedBlockerId: "",
};

export interface TaskInspectorWorkflowCommands {
  operationState: OperationState;
  taskEndsAtControlState: TaskEndsAtControlState;
  taskDependencyControlState: TaskDependencyControlState;
  onEditTask: (taskId: string, edit: TaskEdit) => Promise<void> | void;
  onSetTaskEndsAt: (taskId: string, eventId: string | null, clear: boolean) => Promise<void> | void;
  onSelectTaskEndsAtEvent: (eventId: string) => void;
  onCreateTaskEndEvent: (taskId: string) => void;
  onSelectTaskDependencyBlocker: (blockerId: string) => void;
  onAddDependency: (taskId: string, blockerId: string) => Promise<void> | void;
  onRemoveDependency: (taskId: string, blockerId: string) => Promise<void> | void;
}

export interface TaskInspectorWorkflowDeps {
  currentProjectRoot: () => string | null;
  refreshProjectState: (project: OpenProjectResult, preferredTaskId?: string | null) => void;
  openCreateEndEvent: (projectRoot: string, taskId: string) => void;
  editTaskAction: TaskEditor;
  addDependencyAction: DependencyAdder;
  removeDependencyAction: DependencyRemover;
  setTaskEndsAtAction: TaskEndsAtSetter;
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

export interface TaskInspectorWorkflow {
  resetControlsForSelectionChange: (selectionChanged: boolean) => void;
  commands: (operationState: OperationState) => TaskInspectorWorkflowCommands;
}

export function createTaskInspectorWorkflow(
  deps: TaskInspectorWorkflowDeps,
): TaskInspectorWorkflow {
  let taskEndsAtControlState: TaskEndsAtControlState = {
    ...emptyTaskEndsAtControlState,
  };
  let taskDependencyControlState: TaskDependencyControlState = {
    ...emptyTaskDependencyControlState,
  };

  const editTask = async (taskId: string, edit: TaskEdit) => {
    const projectRoot = deps.currentProjectRoot();
    if (!projectRoot) {
      return;
    }

    try {
      const result = await deps.editTaskAction(projectRoot, taskId, edit);
      deps.refreshProjectState(result.project, taskId);
      deps.setOperationIdle();
    } catch (error) {
      deps.setActionError(error);
      deps.render();
      throw error;
    }
    deps.render();
  };

  const setTaskEndsAt = async (taskId: string, eventId: string | null, clear: boolean) => {
    const projectRoot = deps.currentProjectRoot();
    if (!projectRoot) {
      return;
    }

    if (!clear && !eventId) {
      taskEndsAtControlState = {
        ...taskEndsAtControlState,
        errorMessage: "Choose an event before setting ends_at.",
      };
      deps.render();
      return;
    }

    taskEndsAtControlState = {
      ...taskEndsAtControlState,
      errorMessage: null,
      submitting: true,
    };
    deps.render();

    try {
      const result = await deps.setTaskEndsAtAction(projectRoot, taskId, eventId, clear);
      deps.refreshProjectState(result.project, taskId);
      deps.setOperationIdle();
      taskEndsAtControlState = {
        ...emptyTaskEndsAtControlState,
        selectedEventId: clear ? "" : (eventId ?? ""),
      };
    } catch (error) {
      taskEndsAtControlState = {
        errorMessage: mutationErrorMessage(error),
        submitting: false,
        selectedEventId: taskEndsAtControlState.selectedEventId,
      };
      deps.setActionError(error);
    }
    deps.render();
  };

  const addDependency = async (taskId: string, blockerId: string) => {
    const projectRoot = deps.currentProjectRoot();
    if (!projectRoot || !blockerId) {
      deps.setActionError(new Error("Choose a dependency target before adding it."));
      deps.render();
      return;
    }

    try {
      const result = await deps.addDependencyAction(projectRoot, taskId, blockerId);
      deps.refreshProjectState(result.project, taskId);
      taskDependencyControlState = { ...emptyTaskDependencyControlState };
      deps.setOperationIdle();
    } catch (error) {
      deps.setActionError(error);
    }
    deps.render();
  };

  const removeDependency = async (taskId: string, blockerId: string) => {
    const projectRoot = deps.currentProjectRoot();
    if (!projectRoot || !blockerId) {
      return;
    }

    try {
      const result = await deps.removeDependencyAction(projectRoot, taskId, blockerId);
      deps.refreshProjectState(result.project, taskId);
      deps.setOperationIdle();
    } catch (error) {
      deps.setActionError(error);
    }
    deps.render();
  };

  return {
    resetControlsForSelectionChange: (selectionChanged) => {
      if (!selectionChanged) {
        return;
      }
      taskEndsAtControlState = { ...emptyTaskEndsAtControlState };
      taskDependencyControlState = { ...emptyTaskDependencyControlState };
    },
    commands: (operationState) => ({
      operationState,
      taskEndsAtControlState,
      taskDependencyControlState,
      onEditTask: editTask,
      onSetTaskEndsAt: setTaskEndsAt,
      onSelectTaskEndsAtEvent: (eventId) => {
        taskEndsAtControlState = {
          ...taskEndsAtControlState,
          selectedEventId: eventId,
          errorMessage: null,
        };
        deps.render();
      },
      onCreateTaskEndEvent: (taskId) => {
        const projectRoot = deps.currentProjectRoot();
        if (!projectRoot) {
          return;
        }
        deps.openCreateEndEvent(projectRoot, taskId);
        deps.render();
      },
      onSelectTaskDependencyBlocker: (blockerId) => {
        taskDependencyControlState = {
          selectedBlockerId: blockerId,
        };
        deps.render();
      },
      onAddDependency: addDependency,
      onRemoveDependency: removeDependency,
    }),
  };
}
