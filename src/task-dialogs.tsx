import { NameMutationDialog } from "./dialog-primitives.tsx";
import type { RenameTaskDialogState, TaskDialogState } from "./shell-dialogs.tsx";

interface TaskDialogProps {
  state: TaskDialogState;
  onTaskNameChange: (taskName: string) => void;
  onCancel: () => void;
  onSubmit: (taskName: string) => void;
}

export function CreateTaskDialog({
  state,
  onTaskNameChange,
  onCancel,
  onSubmit,
}: TaskDialogProps) {
  if (!state.open) {
    return null;
  }

  const title =
    state.mode === "add-before" && state.selectedTaskId
      ? `Add task before ${state.selectedTaskId}`
      : state.mode === "add-after" && state.selectedTaskId
        ? `Add task after ${state.selectedTaskId}`
        : "New Task";

  return (
    <NameMutationDialog
      dialogTestId="new-task-dialog-overlay"
      formTestId="new-task-form"
      formClassName="new-task-form"
      inputTestId="new-task-name"
      errorTestId="new-task-error"
      titleId="new-task-dialog-title"
      errorId="new-task-error"
      title={title}
      fieldName="task-name"
      inputLabel="Task name"
      value={state.taskName}
      errorMessage={state.errorMessage}
      submitting={state.submitting}
      cancelTestId="cancel-new-task"
      submitTestId="create-new-task"
      submitLabel="Create Task"
      submittingLabel="Creating Task..."
      onNameChange={onTaskNameChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}

interface RenameTaskDialogProps {
  state: RenameTaskDialogState;
  onTaskNameChange: (taskName: string) => void;
  onCancel: () => void;
  onSubmit: (taskName: string) => void;
}

export function RenameTaskDialog({
  state,
  onTaskNameChange,
  onCancel,
  onSubmit,
}: RenameTaskDialogProps) {
  if (!state.open) {
    return null;
  }

  return (
    <NameMutationDialog
      dialogTestId="rename-task-dialog-overlay"
      formTestId="rename-task-form"
      formClassName="new-task-form"
      inputTestId="rename-task-name"
      errorTestId="rename-task-error"
      titleId="rename-task-dialog-title"
      errorId="rename-task-error"
      title="Rename Task"
      fieldName="task-name"
      inputLabel="Task name"
      value={state.taskName}
      errorMessage={state.errorMessage}
      submitting={state.submitting}
      cancelTestId="cancel-rename-task"
      submitTestId="rename-selected-task"
      submitLabel="Rename Task"
      submittingLabel="Renaming Task..."
      onNameChange={onTaskNameChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}
