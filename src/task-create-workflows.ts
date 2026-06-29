import type {
  TaskActionResult,
  TaskCreator,
  TaskRelativeInserter,
} from "./shell-types.ts";

interface TaskCreateWorkflowActions {
  createTaskAction: TaskCreator;
  insertTaskRelativeAction: TaskRelativeInserter;
}

export type TaskCreateMode = "create" | "add-before" | "add-after";

export async function createTaskBeforeSelectedTask(
  projectRoot: string,
  selectedTaskId: string,
  newTaskId: string,
  actions: TaskCreateWorkflowActions,
): Promise<TaskActionResult> {
  return actions.insertTaskRelativeAction(projectRoot, "before", selectedTaskId, newTaskId);
}

export async function createTaskAfterSelectedTask(
  projectRoot: string,
  selectedTaskId: string,
  newTaskId: string,
  actions: TaskCreateWorkflowActions,
): Promise<TaskActionResult> {
  return actions.insertTaskRelativeAction(projectRoot, "after", selectedTaskId, newTaskId);
}

export async function createTaskForDialog(
  projectRoot: string,
  mode: TaskCreateMode | undefined,
  selectedTaskId: string | undefined,
  newTaskId: string,
  actions: TaskCreateWorkflowActions,
): Promise<TaskActionResult> {
  if (mode === "add-before") {
    if (!selectedTaskId) {
      throw new Error("Adding a task before another task requires a selected task.");
    }
    return createTaskBeforeSelectedTask(projectRoot, selectedTaskId, newTaskId, actions);
  }
  if (mode === "add-after") {
    if (!selectedTaskId) {
      throw new Error("Adding a task after another task requires a selected task.");
    }
    return createTaskAfterSelectedTask(projectRoot, selectedTaskId, newTaskId, actions);
  }
  return actions.createTaskAction(projectRoot, newTaskId);
}
